package com.lumelo.provisioning;

import android.annotation.SuppressLint;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothSocket;
import android.os.Handler;
import android.os.Looper;

import org.json.JSONException;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.util.UUID;

final class ClassicBluetoothTransport {
    private static final String ACK_CREDENTIALS_RECEIVED = "credentials received";
    private static final String ACK_APPLY_STARTED = "apply started";
    private static final long ACK_TIMEOUT_MS = 8_000;
    private static final long AUTO_RECONNECT_DELAY_MS = 1_500;
    private static final long POST_STATUS_RECONNECT_GRACE_MS = 10_000;
    private static final int MAX_ACK_RETRIES = 1;
    private static final int MAX_AUTO_RECONNECTS = 1;

    private enum AckWaitState {
        IDLE,
        CREDENTIALS,
        APPLY
    }

    private enum ProvisioningStage {
        IDLE,
        WAITING_CREDENTIAL_ACK,
        WAITING_APPLY_ACK,
        WAITING_STATUS
    }

    private static final class PendingProvisioning {
        final String ssid;
        final String password;

        PendingProvisioning(String ssid, String password) {
            this.ssid = ssid;
            this.password = password;
        }
    }

    interface Listener {
        void onClassicConnected(BluetoothDevice device);

        void onClassicReconnecting(String message);

        void onClassicDisconnected(String message);

        void onClassicDeviceInfo(String payload);

        void onClassicStatus(String payload);

        void onClassicError(String message);

        void onClassicLog(String message);
    }

    private static final UUID SPP_UUID =
            UUID.fromString("00001101-0000-1000-8000-00805F9B34FB");
    private static final int RFCOMM_CHANNEL = 1;

    private final Listener listener;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());

    private BluetoothSocket socket;
    private BluetoothSocket pendingSocket;
    private BufferedWriter writer;
    private Thread readerThread;
    private volatile boolean disconnectRequested;
    private volatile boolean recoveryCloseRequested;
    private volatile ProvisioningSecurity.Session credentialSecuritySession;
    private volatile boolean applyPendingAfterCredentialAck;
    private volatile boolean connectInProgress;
    private AckWaitState ackWaitState = AckWaitState.IDLE;
    private Runnable ackTimeoutRunnable;
    private BluetoothDevice activeDevice;
    private PendingProvisioning pendingProvisioning;
    private ProvisioningStage provisioningStage = ProvisioningStage.IDLE;
    private int credentialAckRetryCount;
    private int applyAckRetryCount;
    private boolean statusRecoveryPending;
    private int autoReconnectAttemptsRemaining = MAX_AUTO_RECONNECTS;
    private long connectAttemptSerial;
    private long reconnectGraceDeadlineMs;

    ClassicBluetoothTransport(Listener listener) {
        this.listener = listener;
    }

    synchronized boolean isConnected() {
        return socket != null && socket.isConnected();
    }

    @SuppressLint("MissingPermission")
    void connect(BluetoothDevice device) {
        long attemptId;
        disconnect();
        synchronized (this) {
            activeDevice = device;
            disconnectRequested = false;
            credentialSecuritySession = null;
            applyPendingAfterCredentialAck = false;
            clearPendingProvisioningLocked();
            clearAckTimeoutLocked();
            reconnectGraceDeadlineMs = 0L;
            connectInProgress = true;
            autoReconnectAttemptsRemaining = MAX_AUTO_RECONNECTS;
            connectAttemptSerial += 1;
            attemptId = connectAttemptSerial;
        }
        new Thread(() -> connectInBackground(device, attemptId), "LumeloClassicConnect").start();
    }

    private void connectInBackground(BluetoothDevice device, long attemptId) {
        IOException lastFailure = null;
        BluetoothSocket candidate = null;
        lastFailure = null;

        candidate = tryConnectAttempt(
                "trying insecure RFCOMM",
                () -> device.createInsecureRfcommSocketToServiceRecord(SPP_UUID),
                attemptId
        );
        if (candidate == null) {
            lastFailure = lastAttemptFailure;
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to secure RFCOMM",
                    () -> device.createRfcommSocketToServiceRecord(SPP_UUID),
                    attemptId
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to insecure RFCOMM channel 1",
                    () -> createChannelSocket(device, "createInsecureRfcommSocket", RFCOMM_CHANNEL),
                    attemptId
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to secure RFCOMM channel 1",
                    () -> createChannelSocket(device, "createRfcommSocket", RFCOMM_CHANNEL),
                    attemptId
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            if (isConnectCanceled(attemptId)) {
                clearConnectInProgress(attemptId);
                return;
            }
            clearConnectInProgress(attemptId);
            String message = formatConnectFailureMessage(lastFailure);
            postError(message);
            return;
        }

        BufferedWriter connectedWriter;
        BufferedReader connectedReader;
        try {
            connectedWriter = new BufferedWriter(
                    new OutputStreamWriter(candidate.getOutputStream(), StandardCharsets.UTF_8)
            );
            connectedReader = new BufferedReader(
                    new InputStreamReader(candidate.getInputStream(), StandardCharsets.UTF_8)
            );
        } catch (IOException exception) {
            closeQuietly(candidate);
            postError("Classic Bluetooth stream setup failed: " + exception.getMessage());
            return;
        }

        synchronized (this) {
            if (disconnectRequested || attemptId != connectAttemptSerial) {
                clearConnectInProgress(attemptId);
                closeQuietly(candidate);
                return;
            }
            socket = candidate;
            writer = connectedWriter;
            connectInProgress = false;
        }

        post(() -> listener.onClassicConnected(device));
        startReaderLoop(candidate, connectedReader);
        requestDeviceInfo();
        requestStatus();
    }

    private interface SocketFactory {
        BluetoothSocket create() throws Exception;
    }

    private IOException lastAttemptFailure;

    private String formatConnectFailureMessage(IOException lastFailure) {
        String detail = "";
        if (lastFailure != null && lastFailure.getMessage() != null) {
            detail = lastFailure.getMessage().trim();
        }
        StringBuilder message = new StringBuilder(
                "Classic Bluetooth connect failed after RFCOMM fallback. "
                        + "Lumelo was discovered, but the provisioning service did not answer."
        );
        if (!detail.isEmpty()) {
            message.append(" Last error: ").append(detail);
        }
        return message.toString();
    }

    private BluetoothSocket tryConnectAttempt(String label, SocketFactory factory, long attemptId) {
        if (isConnectCanceled(attemptId)) {
            return null;
        }
        BluetoothSocket candidate = null;
        try {
            postLog("Classic Bluetooth connect: " + label);
            candidate = factory.create();
            synchronized (this) {
                if (disconnectRequested || attemptId != connectAttemptSerial) {
                    closeQuietly(candidate);
                    return null;
                }
                pendingSocket = candidate;
            }
            candidate.connect();
            synchronized (this) {
                if (pendingSocket == candidate) {
                    pendingSocket = null;
                }
            }
            if (isConnectCanceled(attemptId)) {
                closeQuietly(candidate);
                return null;
            }
            lastAttemptFailure = null;
            return candidate;
        } catch (Exception exception) {
            synchronized (this) {
                if (pendingSocket == candidate) {
                    pendingSocket = null;
                }
            }
            closeQuietly(candidate);
            lastAttemptFailure = toIoException(label, exception);
            postLog("Classic Bluetooth connect attempt failed (" + label + "): "
                    + lastAttemptFailure.getMessage());
            return null;
        }
    }

    private boolean isConnectCanceled(long attemptId) {
        synchronized (this) {
            return disconnectRequested || attemptId != connectAttemptSerial;
        }
    }

    private void clearConnectInProgress(long attemptId) {
        synchronized (this) {
            if (attemptId == connectAttemptSerial) {
                connectInProgress = false;
            }
        }
    }

    synchronized boolean isConnectInProgress() {
        return connectInProgress;
    }

    private BluetoothSocket createChannelSocket(BluetoothDevice device, String methodName, int channel)
            throws Exception {
        Method method = BluetoothDevice.class.getMethod(methodName, int.class);
        Object result = method.invoke(device, channel);
        if (!(result instanceof BluetoothSocket)) {
            throw new IOException("Hidden RFCOMM method returned no BluetoothSocket");
        }
        return (BluetoothSocket) result;
    }

    private IOException toIoException(String label, Exception exception) {
        if (exception instanceof IOException) {
            return (IOException) exception;
        }
        Throwable cause = exception.getCause();
        if (cause instanceof IOException) {
            return (IOException) cause;
        }
        String message = exception.getMessage();
        if (message == null || message.isEmpty()) {
            message = label + " failed";
        }
        return new IOException(message, exception);
    }

    private void startReaderLoop(BluetoothSocket activeSocket, BufferedReader reader) {
        readerThread = new Thread(() -> readLoop(activeSocket, reader), "LumeloClassicRead");
        readerThread.start();
    }

    private void readLoop(BluetoothSocket activeSocket, BufferedReader reader) {
        String reconnectMessage = "Classic Bluetooth session lost. Reconnecting to T4...";
        try {
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.trim().isEmpty()) {
                    continue;
                }
                handleIncomingLine(line);
            }
        } catch (IOException exception) {
            if (!disconnectRequested && !recoveryCloseRequested) {
                postError("Classic Bluetooth read failed: " + exception.getMessage());
            }
        } finally {
            boolean notifyDisconnect = !disconnectRequested;
            boolean shouldReconnect = false;
            long reconnectAttemptId = 0L;
            BluetoothDevice reconnectDevice = null;
            synchronized (this) {
                boolean recoveryCloseActive = recoveryCloseRequested;
                recoveryCloseRequested = false;
                if (socket == activeSocket) {
                    socket = null;
                    writer = null;
                }
                connectInProgress = false;
                if (notifyDisconnect) {
                    shouldReconnect = shouldAutoReconnectLocked();
                    if (shouldReconnect) {
                        reconnectMessage = recoveryCloseActive
                                ? "Classic Bluetooth write failed. Reconnecting to T4..."
                                : "Classic Bluetooth session lost. Reconnecting to T4...";
                        reconnectAttemptId = beginReconnectLocked(reconnectMessage);
                        reconnectDevice = activeDevice;
                        notifyDisconnect = false;
                    }
                }
            }
            credentialSecuritySession = null;
            applyPendingAfterCredentialAck = false;
            if (!shouldReconnect) {
                clearAckTimeout();
            }
            closeQuietly(activeSocket);
            if (shouldReconnect) {
                scheduleReconnectAttempt(
                        reconnectDevice,
                        reconnectAttemptId,
                        reconnectMessage
                );
            }
            if (notifyDisconnect) {
                postDisconnected("Disconnected from T4.");
            }
        }
    }

    private void handleIncomingLine(String line) {
        try {
            JSONObject message = new JSONObject(line);
            String type = message.optString("type");
            if ("hello".equals(type)) {
                try {
                    JSONObject security = message.optJSONObject("security");
                    if (security != null) {
                        credentialSecuritySession = ProvisioningSecurity.parseSession(
                                security.optString("session_id"),
                                security.optString("scheme"),
                                security.optString("dh_group"),
                                security.optString("server_nonce"),
                                security.optString("server_public_key")
                        );
                    } else {
                        credentialSecuritySession = null;
                    }
                    if (credentialSecuritySession != null) {
                        postLog("Classic Bluetooth credential security negotiated: "
                                + ProvisioningSecurity.CREDENTIAL_SCHEME);
                    }
                } catch (GeneralSecurityException securityException) {
                    credentialSecuritySession = null;
                    postError("Classic Bluetooth security negotiation failed: "
                            + securityException.getMessage());
                }
                postLog("Classic Bluetooth hello: " + line);
                return;
            }
            if ("ack".equals(type)) {
                String ackMessage = message.optString("message");
                boolean shouldSendApply = false;
                if (ACK_CREDENTIALS_RECEIVED.equals(ackMessage) && applyPendingAfterCredentialAck) {
                    clearAckTimeout(AckWaitState.CREDENTIALS);
                    applyPendingAfterCredentialAck = false;
                    provisioningStage = ProvisioningStage.WAITING_APPLY_ACK;
                    statusRecoveryPending = false;
                    shouldSendApply = true;
                } else if (ACK_APPLY_STARTED.equals(ackMessage)) {
                    clearAckTimeout(AckWaitState.APPLY);
                    applyPendingAfterCredentialAck = false;
                    provisioningStage = ProvisioningStage.WAITING_STATUS;
                    statusRecoveryPending = false;
                }
                postLog("Classic Bluetooth ack: " + ackMessage);
                if (shouldSendApply) {
                    if (sendApplyCommand()) {
                        provisioningStage = ProvisioningStage.WAITING_APPLY_ACK;
                    }
                }
                return;
            }
            if ("error".equals(type)) {
                clearAckTimeout();
                applyPendingAfterCredentialAck = false;
                clearPendingProvisioning();
                String errorMessage = message.optString("message");
                if (errorMessage.isEmpty()) {
                    errorMessage = line;
                }
                postError(errorMessage);
                return;
            }

            JSONObject payload = message.optJSONObject("payload");
            if ("device_info".equals(type) && payload != null) {
                post(() -> listener.onClassicDeviceInfo(payload.toString()));
                return;
            }
            if ("status".equals(type) && payload != null) {
                handleStatusPayload(payload);
                post(() -> listener.onClassicStatus(payload.toString()));
                return;
            }

            postLog("Classic Bluetooth message: " + line);
        } catch (JSONException exception) {
            postLog("Classic Bluetooth non-JSON line: " + line);
        }
    }

    boolean requestDeviceInfo() {
        return sendSimpleCommand("device_info");
    }

    boolean requestStatus() {
        return sendSimpleCommand("status");
    }

    void sendCredentials(String ssid, String password) {
        PendingProvisioning provisioning = new PendingProvisioning(ssid, password);
        synchronized (this) {
            pendingProvisioning = provisioning;
            provisioningStage = ProvisioningStage.WAITING_CREDENTIAL_ACK;
            credentialAckRetryCount = 0;
            applyAckRetryCount = 0;
            statusRecoveryPending = false;
            autoReconnectAttemptsRemaining = MAX_AUTO_RECONNECTS;
        }
        if (!sendEncryptedCredentials(provisioning)) {
            clearPendingProvisioning();
        }
    }

    private boolean sendEncryptedCredentials(PendingProvisioning provisioning) {
        try {
            ProvisioningSecurity.Session securitySession = credentialSecuritySession;
            if (securitySession == null) {
                postError("Classic Bluetooth secure credential transport is unavailable. "
                        + "Update the T4 provisioning daemon before sending Wi-Fi credentials.");
                return false;
            }

            ProvisioningSecurity.EncryptedPayload encryptedPayload =
                    ProvisioningSecurity.encryptCredentials(securitySession, provisioning.ssid, provisioning.password);
            JSONObject secureMessage = new JSONObject();
            JSONObject payload = new JSONObject();
            payload.put("scheme", encryptedPayload.scheme);
            payload.put("dh_group", encryptedPayload.dhGroup);
            payload.put("session_id", encryptedPayload.sessionId);
            payload.put("client_public_key", encryptedPayload.clientPublicKey);
            payload.put("client_nonce", encryptedPayload.clientNonce);
            payload.put("message_nonce", encryptedPayload.messageNonce);
            payload.put("ciphertext", encryptedPayload.ciphertext);
            payload.put("mac", encryptedPayload.mac);
            secureMessage.put("type", "wifi_credentials_encrypted");
            secureMessage.put("payload", payload);
            applyPendingAfterCredentialAck = true;
            if (!sendMessage(secureMessage)) {
                applyPendingAfterCredentialAck = false;
                return false;
            }
            scheduleAckTimeout(AckWaitState.CREDENTIALS);
            postLog("Classic Bluetooth credential transport: encrypted payload queued for SSID "
                    + provisioning.ssid);
            return true;
        } catch (JSONException | GeneralSecurityException exception) {
            clearAckTimeout();
            applyPendingAfterCredentialAck = false;
            postError("Failed to build Classic Bluetooth credential payload: " + exception.getMessage());
            return false;
        }
    }

    void disconnect() {
        disconnectRequested = true;
        BluetoothSocket currentSocket;
        BluetoothSocket currentPendingSocket;
        synchronized (this) {
            currentSocket = socket;
            currentPendingSocket = pendingSocket;
            socket = null;
            pendingSocket = null;
            writer = null;
            connectInProgress = false;
            connectAttemptSerial += 1;
            activeDevice = null;
            reconnectGraceDeadlineMs = 0L;
            clearPendingProvisioningLocked();
        }
        credentialSecuritySession = null;
        applyPendingAfterCredentialAck = false;
        clearAckTimeout();
        closeQuietly(currentSocket);
        closeQuietly(currentPendingSocket);
    }

    private boolean sendApplyCommand() {
        if (!sendSimpleCommand("apply")) {
            return false;
        }
        scheduleAckTimeout(AckWaitState.APPLY);
        return true;
    }

    private boolean isReconnectGraceActiveLocked() {
        return reconnectGraceDeadlineMs > System.currentTimeMillis();
    }

    private boolean shouldAutoReconnectLocked() {
        return !disconnectRequested
                && activeDevice != null
                && (isReconnectGraceActiveLocked()
                || (pendingProvisioning != null && provisioningStage != ProvisioningStage.IDLE))
                && autoReconnectAttemptsRemaining > 0;
    }

    private long beginReconnectLocked(String message) {
        autoReconnectAttemptsRemaining -= 1;
        clearAckTimeoutLocked();
        credentialSecuritySession = null;
        applyPendingAfterCredentialAck = false;
        statusRecoveryPending = true;
        connectInProgress = true;
        connectAttemptSerial += 1;
        postLog(message);
        return connectAttemptSerial;
    }

    private void clearPendingProvisioning() {
        synchronized (this) {
            clearPendingProvisioningLocked();
        }
    }

    private void clearPendingProvisioningLocked() {
        pendingProvisioning = null;
        provisioningStage = ProvisioningStage.IDLE;
        credentialAckRetryCount = 0;
        applyAckRetryCount = 0;
        statusRecoveryPending = false;
    }

    private void scheduleAckTimeout(AckWaitState state) {
        Runnable timeoutRunnable = () -> handleAckTimeout(state);
        synchronized (this) {
            clearAckTimeoutLocked();
            ackWaitState = state;
            ackTimeoutRunnable = timeoutRunnable;
            mainHandler.postDelayed(timeoutRunnable, ACK_TIMEOUT_MS);
        }
    }

    private void clearAckTimeout() {
        synchronized (this) {
            clearAckTimeoutLocked();
        }
    }

    private void clearAckTimeout(AckWaitState expectedState) {
        synchronized (this) {
            if (ackWaitState != expectedState) {
                return;
            }
            clearAckTimeoutLocked();
        }
    }

    private void clearAckTimeoutLocked() {
        if (ackTimeoutRunnable != null) {
            mainHandler.removeCallbacks(ackTimeoutRunnable);
            ackTimeoutRunnable = null;
        }
        ackWaitState = AckWaitState.IDLE;
    }

    private void handleAckTimeout(AckWaitState expectedState) {
        PendingProvisioning provisioningToRetry = null;
        boolean shouldRetryApply = false;
        boolean shouldRequestStatus = false;
        boolean shouldReconnect = false;
        long reconnectAttemptId = 0L;
        synchronized (this) {
            if (ackWaitState != expectedState || ackTimeoutRunnable == null) {
                return;
            }
            ackTimeoutRunnable = null;
            ackWaitState = AckWaitState.IDLE;
            applyPendingAfterCredentialAck = false;
            if (expectedState == AckWaitState.CREDENTIALS
                    && pendingProvisioning != null
                    && credentialAckRetryCount < MAX_ACK_RETRIES) {
                credentialAckRetryCount += 1;
                provisioningToRetry = pendingProvisioning;
            } else if (expectedState == AckWaitState.APPLY && applyAckRetryCount < MAX_ACK_RETRIES) {
                applyAckRetryCount += 1;
                shouldRetryApply = true;
            } else if (shouldAutoReconnectLocked()) {
                reconnectAttemptId = beginReconnectLocked(
                        "Classic Bluetooth ack timeout. Reconnecting to T4..."
                );
                shouldReconnect = true;
            } else {
                statusRecoveryPending = true;
                shouldRequestStatus = true;
            }
        }
        String expectedAck = expectedState == AckWaitState.CREDENTIALS
                ? ACK_CREDENTIALS_RECEIVED
                : ACK_APPLY_STARTED;
        if (provisioningToRetry != null) {
            postLog("Classic Bluetooth ack timeout for " + expectedAck + "; retrying encrypted credentials");
            if (sendEncryptedCredentials(provisioningToRetry)) {
                return;
            }
            shouldRequestStatus = true;
        }
        if (shouldRetryApply) {
            postLog("Classic Bluetooth ack timeout for " + expectedAck + "; retrying apply");
            if (sendApplyCommand()) {
                return;
            }
            shouldRequestStatus = true;
        }
        if (shouldReconnect) {
            postError("Classic Bluetooth timed out waiting for ack: " + expectedAck);
            BluetoothDevice reconnectDevice;
            synchronized (this) {
                reconnectDevice = activeDevice;
            }
            scheduleReconnectAttempt(
                    reconnectDevice,
                    reconnectAttemptId,
                    "Classic Bluetooth ack timeout. Reconnecting to T4..."
            );
            return;
        }
        postError("Classic Bluetooth timed out waiting for ack: " + expectedAck);
        if (shouldRequestStatus && !requestStatus()) {
            postLog("Classic Bluetooth recovery status request could not start after ack timeout");
        }
    }

    private void scheduleReconnectAttempt(BluetoothDevice device, long attemptId, String message) {
        post(() -> listener.onClassicReconnecting(message));
        if (device == null) {
            return;
        }
        mainHandler.postDelayed(
                () -> new Thread(
                        () -> connectInBackground(device, attemptId),
                        "LumeloClassicReconnect"
                ).start(),
                AUTO_RECONNECT_DELAY_MS
        );
    }

    private void handleStatusPayload(JSONObject payload) {
        String state = payload.optString("state");
        if (state.isEmpty()) {
            return;
        }

        boolean shouldSendCredentials = false;
        boolean shouldSendApply = false;
        PendingProvisioning provisioning = null;

        synchronized (this) {
            if ("connected".equals(state) || "failed".equals(state)) {
                clearAckTimeoutLocked();
                applyPendingAfterCredentialAck = false;
                reconnectGraceDeadlineMs = System.currentTimeMillis() + POST_STATUS_RECONNECT_GRACE_MS;
                autoReconnectAttemptsRemaining = MAX_AUTO_RECONNECTS;
                clearPendingProvisioningLocked();
                return;
            }
            if ("applying".equals(state) || "waiting_for_ip".equals(state)) {
                clearAckTimeoutLocked();
                applyPendingAfterCredentialAck = false;
                provisioningStage = ProvisioningStage.WAITING_STATUS;
                statusRecoveryPending = false;
                return;
            }
            if (pendingProvisioning == null) {
                return;
            }

            provisioning = pendingProvisioning;
            if ("credentials_ready".equals(state)) {
                clearAckTimeoutLocked();
                applyPendingAfterCredentialAck = false;
                if (provisioningStage == ProvisioningStage.WAITING_CREDENTIAL_ACK) {
                    shouldSendApply = true;
                    provisioningStage = ProvisioningStage.WAITING_APPLY_ACK;
                    statusRecoveryPending = false;
                } else if (statusRecoveryPending
                        && (provisioningStage == ProvisioningStage.WAITING_CREDENTIAL_ACK
                        || provisioningStage == ProvisioningStage.WAITING_APPLY_ACK
                        || provisioningStage == ProvisioningStage.WAITING_STATUS)) {
                    shouldSendApply = true;
                    provisioningStage = ProvisioningStage.WAITING_APPLY_ACK;
                    statusRecoveryPending = false;
                }
            }

            if (statusRecoveryPending
                    && ("idle".equals(state) || "advertising".equals(state))
                    && provisioningStage != ProvisioningStage.IDLE) {
                shouldSendCredentials = true;
                provisioningStage = ProvisioningStage.WAITING_CREDENTIAL_ACK;
                statusRecoveryPending = false;
            }
        }

        if (shouldSendApply) {
            postLog("Classic Bluetooth recovery: credentials are already present; sending apply");
            sendApplyCommand();
            return;
        }
        if (shouldSendCredentials && provisioning != null) {
            postLog("Classic Bluetooth recovery: resending encrypted credentials after status check");
            sendEncryptedCredentials(provisioning);
        }
    }

    synchronized String debugSummary() {
        String deviceLabel = activeDevice == null ? "(none)" : activeDevice.getAddress();
        return "connected=" + isConnected()
                + ", connectInProgress=" + connectInProgress
                + ", stage=" + provisioningStage
                + ", ackWait=" + ackWaitState
                + ", pendingProvisioning=" + (pendingProvisioning != null ? "yes" : "no")
                + ", credentialRetries=" + credentialAckRetryCount + "/" + MAX_ACK_RETRIES
                + ", applyRetries=" + applyAckRetryCount + "/" + MAX_ACK_RETRIES
                + ", reconnectsLeft=" + autoReconnectAttemptsRemaining
                + ", reconnectGraceActive=" + isReconnectGraceActiveLocked()
                + ", statusRecoveryPending=" + statusRecoveryPending
                + ", device=" + deviceLabel;
    }

    private boolean sendSimpleCommand(String type) {
        try {
            JSONObject message = new JSONObject();
            message.put("type", type);
            return sendMessage(message);
        } catch (JSONException exception) {
            postError("Failed to build Classic Bluetooth command: " + type);
            return false;
        }
    }

    private boolean sendMessage(JSONObject message) {
        BufferedWriter currentWriter;
        synchronized (this) {
            currentWriter = writer;
        }
        if (currentWriter == null) {
            postError("Classic Bluetooth session is not connected.");
            return false;
        }

        try {
            currentWriter.write(message.toString());
            currentWriter.write("\n");
            currentWriter.flush();
            postLog("Classic Bluetooth send: " + message.optString("type"));
            return true;
        } catch (IOException exception) {
            postError("Classic Bluetooth write failed: " + exception.getMessage());
            BluetoothSocket currentSocket;
            synchronized (this) {
                recoveryCloseRequested = true;
                currentSocket = socket;
                writer = null;
            }
            closeQuietly(currentSocket);
            return false;
        }
    }

    private void postDisconnected(String message) {
        post(() -> listener.onClassicDisconnected(message));
    }

    private void postError(String message) {
        post(() -> listener.onClassicError(message));
    }

    private void postLog(String message) {
        post(() -> listener.onClassicLog(message));
    }

    private void post(Runnable action) {
        mainHandler.post(action);
    }

    private void closeQuietly(BluetoothSocket currentSocket) {
        if (currentSocket == null) {
            return;
        }
        try {
            currentSocket.close();
        } catch (IOException exception) {
            postLog("Classic Bluetooth socket close failed: " + exception.getClass().getSimpleName());
        }
    }
}
