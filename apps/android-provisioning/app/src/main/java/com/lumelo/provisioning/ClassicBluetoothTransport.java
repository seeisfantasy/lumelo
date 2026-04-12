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
    interface Listener {
        void onClassicConnected(BluetoothDevice device);

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
    private BufferedWriter writer;
    private Thread readerThread;
    private volatile boolean disconnectRequested;
    private volatile ProvisioningSecurity.Session credentialSecuritySession;

    ClassicBluetoothTransport(Listener listener) {
        this.listener = listener;
    }

    synchronized boolean isConnected() {
        return socket != null && socket.isConnected();
    }

    @SuppressLint("MissingPermission")
    void connect(BluetoothDevice device) {
        disconnect();
        disconnectRequested = false;
        credentialSecuritySession = null;
        new Thread(() -> connectInBackground(device), "LumeloClassicConnect").start();
    }

    private void connectInBackground(BluetoothDevice device) {
        IOException lastFailure = null;
        BluetoothSocket candidate = null;
        lastFailure = null;

        candidate = tryConnectAttempt(
                "trying insecure RFCOMM",
                () -> device.createInsecureRfcommSocketToServiceRecord(SPP_UUID)
        );
        if (candidate == null) {
            lastFailure = lastAttemptFailure;
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to secure RFCOMM",
                    () -> device.createRfcommSocketToServiceRecord(SPP_UUID)
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to insecure RFCOMM channel 1",
                    () -> createChannelSocket(device, "createInsecureRfcommSocket", RFCOMM_CHANNEL)
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            candidate = tryConnectAttempt(
                    "falling back to secure RFCOMM channel 1",
                    () -> createChannelSocket(device, "createRfcommSocket", RFCOMM_CHANNEL)
            );
            if (candidate == null) {
                lastFailure = lastAttemptFailure;
            }
        }

        if (candidate == null) {
            String message = "Classic Bluetooth connect failed";
            if (lastFailure != null && lastFailure.getMessage() != null && !lastFailure.getMessage().isEmpty()) {
                message += ": " + lastFailure.getMessage();
            }
            postError(message);
            postDisconnected("Disconnected from T4.");
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
            postDisconnected("Disconnected from T4.");
            return;
        }

        synchronized (this) {
            socket = candidate;
            writer = connectedWriter;
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

    private BluetoothSocket tryConnectAttempt(String label, SocketFactory factory) {
        BluetoothSocket candidate = null;
        try {
            postLog("Classic Bluetooth connect: " + label);
            candidate = factory.create();
            candidate.connect();
            lastAttemptFailure = null;
            return candidate;
        } catch (Exception exception) {
            closeQuietly(candidate);
            lastAttemptFailure = toIoException(label, exception);
            postLog("Classic Bluetooth connect attempt failed (" + label + "): "
                    + lastAttemptFailure.getMessage());
            return null;
        }
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
        try {
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.trim().isEmpty()) {
                    continue;
                }
                handleIncomingLine(line);
            }
        } catch (IOException exception) {
            if (!disconnectRequested) {
                postError("Classic Bluetooth read failed: " + exception.getMessage());
            }
        } finally {
            boolean notifyDisconnect = !disconnectRequested;
            synchronized (this) {
                if (socket == activeSocket) {
                    socket = null;
                    writer = null;
                }
            }
            credentialSecuritySession = null;
            closeQuietly(activeSocket);
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
                postLog("Classic Bluetooth ack: " + message.optString("message"));
                return;
            }
            if ("error".equals(type)) {
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
                post(() -> listener.onClassicStatus(payload.toString()));
                return;
            }

            postLog("Classic Bluetooth message: " + line);
        } catch (JSONException exception) {
            postLog("Classic Bluetooth non-JSON line: " + line);
        }
    }

    void requestDeviceInfo() {
        sendSimpleCommand("device_info");
    }

    void requestStatus() {
        sendSimpleCommand("status");
    }

    void sendCredentials(String ssid, String password) {
        try {
            ProvisioningSecurity.Session securitySession = credentialSecuritySession;
            if (securitySession == null) {
                postError("Classic Bluetooth secure credential transport is unavailable. "
                        + "Update the T4 provisioning daemon before sending Wi-Fi credentials.");
                return;
            }

            ProvisioningSecurity.EncryptedPayload encryptedPayload =
                    ProvisioningSecurity.encryptCredentials(securitySession, ssid, password);
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
            sendMessage(secureMessage);
            postLog("Classic Bluetooth credential transport: encrypted payload queued for SSID " + ssid);
            sendSimpleCommand("apply");
        } catch (JSONException | GeneralSecurityException exception) {
            postError("Failed to build Classic Bluetooth credential payload: " + exception.getMessage());
        }
    }

    void disconnect() {
        disconnectRequested = true;
        BluetoothSocket currentSocket;
        synchronized (this) {
            currentSocket = socket;
            socket = null;
            writer = null;
        }
        closeQuietly(currentSocket);
    }

    private void sendSimpleCommand(String type) {
        try {
            JSONObject message = new JSONObject();
            message.put("type", type);
            sendMessage(message);
        } catch (JSONException exception) {
            postError("Failed to build Classic Bluetooth command: " + type);
        }
    }

    private void sendMessage(JSONObject message) {
        BufferedWriter currentWriter;
        synchronized (this) {
            currentWriter = writer;
        }
        if (currentWriter == null) {
            postError("Classic Bluetooth session is not connected.");
            return;
        }

        try {
            currentWriter.write(message.toString());
            currentWriter.write("\n");
            currentWriter.flush();
            postLog("Classic Bluetooth send: " + message.optString("type"));
        } catch (IOException exception) {
            postError("Classic Bluetooth write failed: " + exception.getMessage());
            disconnect();
            postDisconnected("Disconnected from T4.");
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
        } catch (IOException ignored) {
        }
    }
}
