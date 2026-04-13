# AI Review Part 03

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java` (2/3)

- bytes: 92720
- segment: 2/3

~~~java
                uuidMatch,
                nameMatch,
                false,
                false,
                listEntry.toString(),
                detail.toString(),
                logLine
        );
    }

    private void refreshScanSummary() {
        int uuidMatched = 0;
        int nameMatched = 0;
        int paired = 0;
        for (ScanObservation observation : scanObservations) {
            if (observation.uuidMatch) {
                uuidMatched++;
            }
            if (observation.nameMatch) {
                nameMatched++;
            }
            if (observation.bonded) {
                paired++;
            }
        }
        String modeLabel = activeScanMode == ScanMode.LUMELO
                ? "Classic Bluetooth scan"
                : "Raw BLE scan";
        String selectedAddress = selectedDevice == null ? "none" : selectedDevice.getAddress();
        scanSummaryView.setText(String.format(
                Locale.US,
                "Scan summary: %s | devices=%d | uuidMatch=%d | nameMatch=%d | paired=%d | selected=%s",
                modeLabel,
                scanObservations.size(),
                uuidMatched,
                nameMatched,
                paired,
                selectedAddress
        ));
    }

    private int countClassicNameMatches() {
        int count = 0;
        for (ScanObservation observation : scanObservations) {
            if (observation.nameMatch) {
                count++;
            }
        }
        return count;
    }

    private boolean hasLumeloServiceUuid(ScanRecord scanRecord) {
        if (scanRecord == null || scanRecord.getServiceUuids() == null) {
            return false;
        }
        for (android.os.ParcelUuid uuid : scanRecord.getServiceUuids()) {
            if (SERVICE_UUID.equals(uuid.getUuid())) {
                return true;
            }
        }
        return false;
    }

    private boolean startsWithLumelo(String value) {
        if (value == null) {
            return false;
        }
        String normalized = value.toLowerCase(Locale.ROOT);
        return normalized.startsWith("lumelo") || normalized.startsWith("nanopc-t4");
    }

    private String sanitize(String value) {
        return value == null ? "" : value.trim();
    }

    @SuppressLint("MissingPermission")
    private String resolveClassicName(BluetoothDevice device, String discoveredName) {
        String candidateName = sanitize(discoveredName);
        if (!candidateName.isEmpty()) {
            return candidateName;
        }
        if (device == null || !hasConnectPermission()) {
            return "";
        }
        String deviceName = sanitize(device.getName());
        if (!deviceName.isEmpty()) {
            return deviceName;
        }
        return rememberedClassicName(device.getAddress());
    }

    private String formatServiceUuids(ScanRecord scanRecord) {
        if (scanRecord.getServiceUuids() == null || scanRecord.getServiceUuids().isEmpty()) {
            return "";
        }
        StringBuilder builder = new StringBuilder();
        for (android.os.ParcelUuid uuid : scanRecord.getServiceUuids()) {
            if (builder.length() > 0) {
                builder.append(", ");
            }
            builder.append(uuid.getUuid());
        }
        return builder.toString();
    }

    private String formatManufacturerData(ScanRecord scanRecord) {
        SparseArray<byte[]> manufacturerData = scanRecord.getManufacturerSpecificData();
        if (manufacturerData == null || manufacturerData.size() == 0) {
            return "";
        }
        StringBuilder builder = new StringBuilder();
        for (int index = 0; index < manufacturerData.size(); index++) {
            if (builder.length() > 0) {
                builder.append(", ");
            }
            int key = manufacturerData.keyAt(index);
            builder.append(String.format(Locale.US, "0x%04X=", key));
            builder.append(bytesToHex(manufacturerData.valueAt(index)));
        }
        return builder.toString();
    }

    private String bytesToHex(byte[] value) {
        if (value == null || value.length == 0) {
            return "";
        }
        StringBuilder builder = new StringBuilder(value.length * 2);
        for (byte b : value) {
            builder.append(String.format(Locale.US, "%02X", b & 0xff));
        }
        return builder.toString();
    }

    private String shorten(String value, int maxLength) {
        if (value == null || value.length() <= maxLength) {
            return value == null ? "" : value;
        }
        return value.substring(0, maxLength - 3) + "...";
    }

    private void appendIndentedLine(StringBuilder builder, String label, String value) {
        if (value == null || value.isEmpty()) {
            return;
        }
        builder.append("\n").append(label).append(": ").append(value);
    }

    @SuppressLint("MissingPermission")
    private void connectSelectedDevice() {
        if (selectedDevice == null) {
            setStatus(activeScanMode == ScanMode.LUMELO
                    ? "Select a Lumelo device first."
                    : "Select a raw BLE device first.");
            logEvent("Connect blocked: no selected device");
            return;
        }
        if (!hasConnectPermission()) {
            logEvent("Connect blocked: missing connect permission");
            requestBlePermissionsIfNeeded();
            return;
        }

        stopScan();
        if (gatt != null) {
            closeGattQuietly(true);
        }
        stopStatusPolling();
        resetProvisioningSession();
        connectionMode = activeScanMode == ScanMode.GENERIC_TEST
                ? ConnectionMode.GENERIC_TEST
                : ConnectionMode.LUMELO;
        negotiatedMtu = DEFAULT_ATT_MTU;
        serviceDiscoveryStarted = false;

        setStatus(connectionMode == ConnectionMode.LUMELO
                ? "Connecting over classic Bluetooth to " + displayName(selectedDevice)
                : "Connecting to raw BLE device " + displayName(selectedDevice));
        logEvent("Connecting to " + displayName(selectedDevice));
        if (connectionMode == ConnectionMode.LUMELO) {
            if (bluetoothAdapter != null && bluetoothAdapter.isDiscovering()) {
                bluetoothAdapter.cancelDiscovery();
            }
            if (classicTransport != null) {
                classicTransport.connect(selectedDevice);
                runOnUiThread(() -> disconnectButton.setEnabled(true));
            }
            return;
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            gatt = selectedDevice.connectGatt(this, false, gattCallback, BluetoothDevice.TRANSPORT_LE);
        } else {
            gatt = selectedDevice.connectGatt(this, false, gattCallback);
        }
        runOnUiThread(() -> disconnectButton.setEnabled(gatt != null));
    }

    private final BluetoothGattCallback gattCallback = new BluetoothGattCallback() {
        @SuppressLint("MissingPermission")
        @Override
        public void onConnectionStateChange(BluetoothGatt bluetoothGatt, int status, int newState) {
            logEvent("Connection state change: status=" + describeGattStatus(status)
                    + ", newState=" + describeConnectionState(newState));
            if (status != BluetoothGatt.GATT_SUCCESS && newState != BluetoothProfile.STATE_CONNECTED) {
                stopStatusPolling();
                finishGattSession(bluetoothGatt);
                setStatus("Connection failed: " + describeGattStatus(status));
                return;
            }
            if (newState == BluetoothProfile.STATE_CONNECTED) {
                negotiatedMtu = DEFAULT_ATT_MTU;
                serviceDiscoveryStarted = false;
                if (requestPreferredMtu(bluetoothGatt)) {
                    setStatus(connectionMode == ConnectionMode.LUMELO
                            ? "Connected. Negotiating BLE MTU before service discovery..."
                            : "Connected. Negotiating BLE MTU before service discovery...");
                    logEvent("Requested BLE MTU " + DESIRED_ATT_MTU);
                    handler.postDelayed(() -> {
                        if (gatt == bluetoothGatt && !serviceDiscoveryStarted) {
                            logEvent("MTU request timed out; starting service discovery with mtu=" + negotiatedMtu);
                            setStatus(connectionMode == ConnectionMode.LUMELO
                                    ? "Connected. MTU negotiation timed out; discovering Lumelo provisioning service..."
                                    : "Connected. MTU negotiation timed out; discovering BLE services...");
                            startServiceDiscovery(bluetoothGatt);
                        }
                    }, MTU_REQUEST_TIMEOUT_MS);
                    return;
                }
                setStatus(connectionMode == ConnectionMode.LUMELO
                        ? "Connected. Discovering Lumelo provisioning service..."
                        : "Connected. Discovering BLE services...");
                logEvent("Connected. MTU request unavailable, starting service discovery");
                startServiceDiscovery(bluetoothGatt);
                return;
            }
            if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                ConnectionMode disconnectedMode = connectionMode;
                stopStatusPolling();
                finishGattSession(bluetoothGatt);
                setStatus(disconnectedMode == ConnectionMode.LUMELO
                        ? "Disconnected from T4."
                        : "Disconnected from raw BLE device.");
            }
        }

        @Override
        public void onMtuChanged(BluetoothGatt bluetoothGatt, int mtu, int status) {
            negotiatedMtu = mtu > 0 ? mtu : DEFAULT_ATT_MTU;
            logEvent("MTU changed: status=" + describeGattStatus(status) + ", mtu=" + negotiatedMtu);
            if (status == BluetoothGatt.GATT_SUCCESS) {
                setStatus(connectionMode == ConnectionMode.LUMELO
                        ? "Connected. Discovering Lumelo provisioning service..."
                        : "Connected. Discovering BLE services...");
            } else {
                setStatus(connectionMode == ConnectionMode.LUMELO
                        ? "Connected. MTU request failed; discovering Lumelo provisioning service..."
                        : "Connected. MTU request failed; discovering BLE services...");
            }
            startServiceDiscovery(bluetoothGatt);
        }

        @Override
        public void onServicesDiscovered(BluetoothGatt bluetoothGatt, int status) {
            logEvent("Services discovered: status=" + describeGattStatus(status));
            if (status != BluetoothGatt.GATT_SUCCESS) {
                setStatus("Service discovery failed: " + describeGattStatus(status));
                return;
            }
            if (connectionMode == ConnectionMode.GENERIC_TEST) {
                showGenericBleServices(bluetoothGatt);
                return;
            }
            BluetoothGattService service = bluetoothGatt.getService(SERVICE_UUID);
            if (service == null) {
                setStatus("Lumelo provisioning service not found.");
                logEvent("Lumelo provisioning service not found");
                return;
            }

            deviceInfoCharacteristic = service.getCharacteristic(DEVICE_INFO_UUID);
            wifiCredentialsCharacteristic = service.getCharacteristic(WIFI_CREDENTIALS_UUID);
            applyCharacteristic = service.getCharacteristic(APPLY_UUID);
            statusCharacteristic = service.getCharacteristic(STATUS_UUID);

            boolean ready = deviceInfoCharacteristic != null
                    && wifiCredentialsCharacteristic != null
                    && applyCharacteristic != null
                    && statusCharacteristic != null;

            runOnUiThread(() -> {
                sendButton.setEnabled(ready);
                readStatusButton.setEnabled(ready);
                disconnectButton.setEnabled(true);
            });
            if (!ready) {
                setStatus("Provisioning service is missing required characteristics.");
                logEvent("Provisioning service missing required characteristics");
                return;
            }

            setStatus("Provisioning service ready.");
            logEvent("Provisioning service ready");
            readDeviceInfo(bluetoothGatt);
        }

        @Override
        public void onCharacteristicRead(
                BluetoothGatt bluetoothGatt,
                BluetoothGattCharacteristic characteristic,
                int status
        ) {
            logEvent("Characteristic read: "
                    + characteristic.getUuid()
                    + ", status="
                    + describeGattStatus(status));
            if (DEVICE_INFO_UUID.equals(characteristic.getUuid())) {
                String text = new String(characteristic.getValue(), StandardCharsets.UTF_8);
                runOnUiThread(() -> deviceInfoView.setText("Device info: " + text));
                logEvent("Device info payload: " + text);
                enableStatusNotifications(bluetoothGatt);
            }
            if (STATUS_UUID.equals(characteristic.getUuid())) {
                showResult(new String(characteristic.getValue(), StandardCharsets.UTF_8));
            }
        }

        @Override
        public void onCharacteristicRead(
                BluetoothGatt bluetoothGatt,
                BluetoothGattCharacteristic characteristic,
                byte[] value,
                int status
        ) {
            logEvent("Characteristic read: "
                    + characteristic.getUuid()
                    + ", status="
                    + describeGattStatus(status));
            if (DEVICE_INFO_UUID.equals(characteristic.getUuid())) {
                String text = new String(value, StandardCharsets.UTF_8);
                runOnUiThread(() -> deviceInfoView.setText("Device info: " + text));
                logEvent("Device info payload: " + text);
                enableStatusNotifications(bluetoothGatt);
            }
            if (STATUS_UUID.equals(characteristic.getUuid())) {
                showResult(new String(value, StandardCharsets.UTF_8));
            }
        }

        @Override
        public void onCharacteristicChanged(
                BluetoothGatt bluetoothGatt,
                BluetoothGattCharacteristic characteristic
        ) {
            if (STATUS_UUID.equals(characteristic.getUuid())) {
                logEvent("Status notification received");
                showResult(new String(characteristic.getValue(), StandardCharsets.UTF_8));
            }
        }

        @Override
        public void onCharacteristicChanged(
                BluetoothGatt bluetoothGatt,
                BluetoothGattCharacteristic characteristic,
                byte[] value
        ) {
            if (STATUS_UUID.equals(characteristic.getUuid())) {
                logEvent("Status notification received");
                showResult(new String(value, StandardCharsets.UTF_8));
            }
        }

        @Override
        public void onCharacteristicWrite(
                BluetoothGatt bluetoothGatt,
                BluetoothGattCharacteristic characteristic,
                int status
        ) {
            logEvent("Characteristic write: "
                    + characteristic.getUuid()
                    + ", status="
                    + describeGattStatus(status));
            if (status != BluetoothGatt.GATT_SUCCESS) {
                setStatus("BLE write failed: " + describeGattStatus(status));
            }
            writeInFlight = false;
            writeNext();
        }

        @Override
        public void onDescriptorWrite(
                BluetoothGatt bluetoothGatt,
                BluetoothGattDescriptor descriptor,
                int status
        ) {
            logEvent("Descriptor write: "
                    + descriptor.getUuid()
                    + ", status="
                    + describeGattStatus(status));
            if (status == BluetoothGatt.GATT_SUCCESS
                    && statusCharacteristic != null
                    && STATUS_UUID.equals(descriptor.getCharacteristic().getUuid())) {
                requestStatusRead(bluetoothGatt);
            }
        }
    };

    @SuppressLint("MissingPermission")
    private void readDeviceInfo(BluetoothGatt bluetoothGatt) {
        if (deviceInfoCharacteristic != null && hasConnectPermission()) {
            boolean started = bluetoothGatt.readCharacteristic(deviceInfoCharacteristic);
            if (!started) {
                logEvent("Device info read could not start, enabling notifications directly");
                enableStatusNotifications(bluetoothGatt);
            }
        }
    }

    @SuppressLint("MissingPermission")
    private void enableStatusNotifications(BluetoothGatt bluetoothGatt) {
        if (statusCharacteristic == null || !hasConnectPermission()) {
            return;
        }
        bluetoothGatt.setCharacteristicNotification(statusCharacteristic, true);
        BluetoothGattDescriptor descriptor = statusCharacteristic.getDescriptor(CLIENT_CONFIG_UUID);
        if (descriptor == null) {
            logEvent("Status characteristic has no CCC descriptor");
            return;
        }
        logEvent("Enabling status notifications");
        if (Build.VERSION.SDK_INT >= 33) {
            bluetoothGatt.writeDescriptor(descriptor, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE);
        } else {
            descriptor.setValue(BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE);
            bluetoothGatt.writeDescriptor(descriptor);
        }
    }

    @SuppressLint("MissingPermission")
    private void requestStatusRead() {
        if (connectionMode == ConnectionMode.LUMELO) {
            if (classicTransport == null || !classicTransport.isConnected()) {
                setStatus("Connect to Lumelo T4 before reading status.");
                logEvent("Status read blocked: classic Bluetooth session unavailable");
                return;
            }
            classicTransport.requestStatus();
            setStatus("Reading T4 status...");
            logEvent("Requested T4 status read over classic Bluetooth");
            return;
        }
        requestStatusRead(gatt);
    }

    @SuppressLint("MissingPermission")
    private void requestStatusRead(BluetoothGatt bluetoothGatt) {
        if (bluetoothGatt == null || statusCharacteristic == null || !hasConnectPermission()) {
            setStatus("Connect to Lumelo T4 before reading status.");
            logEvent("Status read blocked: GATT status characteristic unavailable");
            return;
        }
        boolean started = bluetoothGatt.readCharacteristic(statusCharacteristic);
        if (!started) {
            setStatus("Status read could not start.");
            logEvent("Status read could not start");
            return;
        }
        setStatus("Reading T4 status...");
        logEvent("Requested T4 status read");
    }

    private void sendWifiCredentials() {
        String ssid = ssidInput.getText().toString().trim();
        String password = passwordInput.getText().toString();
        if (ssid.isEmpty()) {
            setStatus("SSID is required.");
            logEvent("Cannot send credentials: empty SSID");
            return;
        }
        if (password.length() < 8 || password.length() > 63) {
            setStatus("WPA password must be 8..63 characters.");
            logEvent("Cannot send credentials: password length invalid");
            return;
        }
        if (connectionMode == ConnectionMode.LUMELO) {
            if (classicTransport == null || !classicTransport.isConnected()) {
                setStatus("Connect to Lumelo T4 before sending credentials.");
                logEvent("Cannot send credentials: classic Bluetooth session not ready");
                return;
            }
            classicTransport.sendCredentials(ssid, password);
            startStatusPolling();
            setStatus("Sent credentials. Waiting for T4 status...");
            logEvent("Queued Wi-Fi credentials for SSID " + ssid + " over classic Bluetooth");
            return;
        }
        if (gatt == null || wifiCredentialsCharacteristic == null || applyCharacteristic == null) {
            setStatus("Connect to Lumelo T4 before sending credentials.");
            logEvent("Cannot send credentials: GATT characteristics not ready");
            return;
        }

        String payload = String.format(
                Locale.US,
                "{\"ssid\":\"%s\",\"password\":\"%s\"}",
                escapeJson(ssid),
                escapeJson(password)
        );
        byte[] payloadBytes = payload.getBytes(StandardCharsets.UTF_8);
        int attPayloadBudget = Math.max(0, negotiatedMtu - 3);
        if (payloadBytes.length > attPayloadBudget) {
            logEvent("Wi-Fi payload is " + payloadBytes.length
                    + " bytes; current ATT payload budget is " + attPayloadBudget
                    + " bytes. Peer must support long writes or a larger MTU.");
        }
        enqueueWrite(wifiCredentialsCharacteristic, payloadBytes);
        enqueueWrite(applyCharacteristic, "{}".getBytes(StandardCharsets.UTF_8));
        startStatusPolling();
        setStatus("Sent credentials. Waiting for T4 status...");
        logEvent("Queued Wi-Fi credentials for SSID " + ssid);
    }

    private void enqueueWrite(BluetoothGattCharacteristic characteristic, byte[] value) {
        writeQueue.add(new WriteRequest(characteristic, value));
        logEvent("Queued write for characteristic " + characteristic.getUuid());
        writeNext();
    }

    @SuppressLint("MissingPermission")
    private void writeNext() {
        if (writeInFlight || writeQueue.isEmpty() || gatt == null || !hasConnectPermission()) {
            return;
        }
        WriteRequest request = writeQueue.remove();
        writeInFlight = true;
        boolean started;
        if (Build.VERSION.SDK_INT >= 33) {
            int status = gatt.writeCharacteristic(
                    request.characteristic,
                    request.value,
                    BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
            );
            started = status == BluetoothGatt.GATT_SUCCESS;
            if (!started) {
                logEvent("writeCharacteristic returned " + describeGattStatus(status));
            }
        } else {
            request.characteristic.setWriteType(BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT);
            request.characteristic.setValue(request.value);
            started = gatt.writeCharacteristic(request.characteristic);
        }
        if (!started) {
            writeInFlight = false;
            setStatus("Failed to start BLE write.");
            logEvent("Failed to start BLE write for characteristic " + request.characteristic.getUuid());
        }
    }

    private boolean hasScanPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            return checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) == PackageManager.PERMISSION_GRANTED;
        }
        return checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED;
    }

    private boolean hasConnectPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            return checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED;
        }
        return true;
    }

    @SuppressLint("MissingPermission")
    private String displayName(BluetoothDevice device) {
        String name = hasConnectPermission() ? device.getName() : null;
        if (name == null || name.isEmpty()) {
            name = rememberedClassicName(device.getAddress());
        }
        if (name == null || name.isEmpty()) {
            name = activeScanMode == ScanMode.GENERIC_TEST ? "BLE device" : "Lumelo device";
        }
        return name + " (" + device.getAddress() + ")";
    }

    @SuppressLint("MissingPermission")
    private void showGenericBleServices(BluetoothGatt bluetoothGatt) {
        List<BluetoothGattService> services = bluetoothGatt.getServices();
        StringBuilder summary = new StringBuilder("Raw BLE result:");
        appendResultLine(summary, "Device", displayName(bluetoothGatt.getDevice()));
        appendResultLine(summary, "Services", String.valueOf(services.size()));

        int limit = Math.min(services.size(), 10);
        for (int index = 0; index < limit; index++) {
            BluetoothGattService service = services.get(index);
            appendResultLine(
                    summary,
                    "Service " + (index + 1),
                    service.getUuid() + " (" + service.getCharacteristics().size() + " chars)"
            );
        }
        if (services.size() > limit) {
            appendResultLine(summary, "More", (services.size() - limit) + " additional service(s)");
        }

        runOnUiThread(() -> {
            deviceInfoView.setText("Device info: raw BLE device connected");
            resultView.setText(summary.toString());
            sendButton.setEnabled(false);
            readStatusButton.setEnabled(false);
            disconnectButton.setEnabled(true);
            openWebButton.setEnabled(false);
            openProvisioningButton.setEnabled(false);
            openLogsButton.setEnabled(false);
            openHealthzButton.setEnabled(false);
        });
        setStatus("Raw BLE device connected. Services discovered.");
        logEvent("Raw BLE service discovery complete: " + services.size() + " service(s)");
    }

    private void showResult(String text) {
        String state = extractJsonString(text, "state");
        String message = extractJsonString(text, "message");
        String ssid = extractJsonString(text, "ssid");
        String ip = extractJsonString(text, "ip");
        String wifiInterface = extractJsonString(text, "wifi_interface");
        String errorCode = extractJsonString(text, "error_code");
        String applyOutput = extractJsonString(text, "apply_output");
        String diagnosticHint = extractJsonString(text, "diagnostic_hint");
        String wpaUnit = extractJsonString(text, "wpa_unit");
        int ipWaitSeconds = extractJsonInt(text, "ip_wait_seconds");
        runOnUiThread(() -> {
            webUrl = extractJsonString(text, "web_url");
            if ((webUrl == null || webUrl.isEmpty()) && ip != null && !ip.isEmpty()) {
                webUrl = "http://" + ip + ":18080/";
            }
            if (webUrl != null && !webUrl.isEmpty()) {
                persistWebUiUrl(webUrl);
            }
            resultView.setText(formatResultText(
                    state,
                    message,
                    ssid,
                    ip,
                    extractJsonString(text, "wifi_ip"),
                    extractJsonString(text, "wired_ip"),
                    wifiInterface,
                    errorCode,
                    applyOutput,
                    diagnosticHint,
                    wpaUnit,
                    ipWaitSeconds,
                    text
            ));
            boolean hasWebUrl = webUrl != null && !webUrl.isEmpty();
            setWebButtonsEnabled(hasWebUrl);
        });
        boolean openedMainInterface = maybeOpenMainInterface(state);
        if ("connected".equals(state) || "failed".equals(state)) {
            stopStatusPolling();
        } else if ("applying".equals(state) || "waiting_for_ip".equals(state) || "credentials_ready".equals(state)) {
            startStatusPolling();
        }
        if (!state.isEmpty() || !message.isEmpty()) {
            String summary = state;
            if (!message.isEmpty()) {
                summary = summary.isEmpty() ? message : summary + " - " + message;
            }
            if ("connected".equals(state) && !openedMainInterface) {
                String networkHint = crossNetworkHint();
                if (!networkHint.isEmpty()) {
                    setStatus("T4 status: " + summary + ". " + networkHint);
                } else {
                    setStatus("T4 status: " + summary);
                }
            } else {
                setStatus("T4 status: " + summary);
            }
        }
        logEvent("Result payload: " + text);
    }

    private void setStatus(String text) {
        runOnUiThread(() -> statusView.setText(text));
        logEvent("Status: " + text);
    }

    private String buildBuildInfoText() {
        return String.format(
                Locale.US,
                "Build: v%s | built %s | git %s",
                BuildConfig.VERSION_NAME,
                BuildConfig.BUILD_TIME_UTC,
                BuildConfig.GIT_SHA_SHORT
        );
    }

    private String buildEnvironmentSummary() {
        String adapterState;
        if (bluetoothAdapter == null) {
            adapterState = "adapter=missing";
        } else if (bluetoothAdapter.isEnabled()) {
            adapterState = "adapter=on";
        } else {
            adapterState = "adapter=off";
        }

        String sessionState;
        if (classicTransport != null && classicTransport.isConnected()) {
            sessionState = "classic_connected";
        } else if (statusCharacteristic != null) {
            sessionState = "gatt_ready";
        } else if (gatt != null) {
            sessionState = "gatt_connected";
        } else {
            sessionState = "idle";
        }

        return String.format(
                Locale.US,
                "Environment: %s, scanPerm=%s, connectPerm=%s, session=%s, mtu=%d, sdk=%d",
                adapterState,
                hasScanPermission() ? "granted" : "missing",
                hasConnectPermission() ? "granted" : "missing",
                sessionState,
                negotiatedMtu,
                Build.VERSION.SDK_INT
        );
    }

    private void refreshEnvironmentStatus() {
        String summary = buildEnvironmentSummary();
        runOnUiThread(() -> environmentView.setText(summary));
    }

    private void logEvent(String line) {
        handler.post(() -> {
            if (line.equals(lastLoggedMessage)) {
                return;
            }
            lastLoggedMessage = line;
            debugLines.addLast(timestampNow() + " " + line);
            while (debugLines.size() > DEBUG_LOG_HISTORY_LIMIT) {
                debugLines.removeFirst();
            }
            refreshDebugLogView();
            refreshEnvironmentStatus();
        });
    }

    private void clearDebugLog() {
        handler.post(() -> {
            debugLines.clear();
            lastLoggedMessage = "debug log cleared";
            debugLines.addLast(timestampNow() + " debug log cleared");
            refreshDebugLogView();
            refreshEnvironmentStatus();
        });
    }

    private void refreshDebugLogView() {
        StringBuilder text = new StringBuilder("Debug log:");
        int skip = Math.max(0, debugLines.size() - DEBUG_LOG_VIEW_LIMIT);
        int index = 0;
        for (String entry : debugLines) {
            if (index++ < skip) {
                continue;
            }
            text.append("\n- ").append(entry);
        }
        debugLogView.setText(text.toString());
    }

    private String timestampNow() {
        return new SimpleDateFormat("HH:mm:ss", Locale.US).format(new Date());
    }

    private void exportDiagnostics() {
        String report = buildDiagnosticReport();
        Intent intent = new Intent(Intent.ACTION_SEND);
        intent.setType("text/plain");
        intent.putExtra(Intent.EXTRA_SUBJECT, "Lumelo setup diagnostic");
        intent.putExtra(Intent.EXTRA_TEXT, report);
        try {
            startActivity(Intent.createChooser(intent, "Export diagnostics"));
            logEvent("Opened diagnostics export sheet");
        } catch (Exception exception) {
            setStatus("No app available to export diagnostics.");
            logEvent("Diagnostics export unavailable: " + exception.getClass().getSimpleName());
        }
    }

    private String buildDiagnosticReport() {
        StringBuilder builder = new StringBuilder();
        builder.append("Lumelo Setup Diagnostic\n");
        appendReportLine(builder, "Build", buildBuildInfoText());
        appendReportLine(builder, "Status", statusView.getText().toString());
        appendReportLine(builder, "Environment", buildEnvironmentSummary());
        appendReportLine(builder, "Selected", selectedView.getText().toString());
        appendReportLine(builder, "Device Info", deviceInfoView.getText().toString());
        appendReportLine(builder, "Result", resultView.getText().toString());
        appendReportLine(builder, "Web URL", webUrl);
        builder.append("\nScan Summary\n");
        builder.append(scanSummaryView.getText()).append('\n');
        builder.append("\nScan Results\n");
        if (scanObservations.isEmpty()) {
            builder.append("- none\n");
        } else {
            for (ScanObservation observation : scanObservations) {
                builder.append("- ").append(observation.detailText.replace("\n", "\n  ")).append("\n");
            }
        }
        builder.append("\nDebug Log\n");
        if (debugLines.isEmpty()) {
            builder.append("- none\n");
        } else {
            for (String line : debugLines) {
                builder.append("- ").append(line).append("\n");
            }
        }
        return builder.toString();
    }

    private void appendReportLine(StringBuilder builder, String label, String value) {
        if (value == null || value.isEmpty()) {
            return;
        }
        builder.append(label).append(": ").append(value).append('\n');
    }

    private void fillCurrentWifiSsid() {
        String currentSsid = currentConnectedWifiSsid();
        if (currentSsid.isEmpty()) {
            setStatus("Current Wi-Fi SSID is unavailable on this phone.");
            logEvent("Current Wi-Fi SSID unavailable");
            return;
        }
        ssidInput.setText(currentSsid);
        setStatus("Filled SSID from current Wi-Fi.");
        logEvent("Filled SSID from current Wi-Fi: " + currentSsid);
    }

    private TextView label(String text) {
        TextView view = new TextView(this);
        view.setText(text);
        view.setTextSize(16);
        view.setTextColor(0xff1f1d1a);
        view.setPadding(0, dp(8), 0, dp(8));
        return view;
    }

    private Button button(String text) {
        Button button = new Button(this);
        button.setText(text);
        return button;
    }

    private EditText input(String hint, boolean password) {
        EditText input = new EditText(this);
        input.setHint(hint);
        input.setSingleLine(true);
        input.setInputType(password
                ? InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD
                : InputType.TYPE_CLASS_TEXT);
        return input;
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
        );
    }

    private int dp(int value) {
        return (int) (value * getResources().getDisplayMetrics().density + 0.5f);
    }

    private String escapeJson(String value) {
        return value
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }

    private String extractJsonString(String json, String key) {
        String needle = "\"" + key + "\":\"";
        int start = json.indexOf(needle);
        if (start < 0) {
            return "";
        }
        start += needle.length();
        StringBuilder value = new StringBuilder();
        boolean escaped = false;
        for (int index = start; index < json.length(); index++) {
            char ch = json.charAt(index);
            if (escaped) {
                value.append(ch);
                escaped = false;
                continue;
            }
            if (ch == '\\') {
                escaped = true;
                continue;
            }
            if (ch == '"') {
                break;
            }
            value.append(ch);
        }
        return value.toString();
    }

    private int extractJsonInt(String json, String key) {
        String needle = "\"" + key + "\":";
        int start = json.indexOf(needle);
        if (start < 0) {
            return 0;
        }
        start += needle.length();
        int end = start;
        while (end < json.length()) {
            char ch = json.charAt(end);
            if ((ch >= '0' && ch <= '9') || ch == '-') {
                end++;
                continue;
            }
            break;
        }
        if (end <= start) {
            return 0;
        }
        try {
            return Integer.parseInt(json.substring(start, end));
        } catch (NumberFormatException ignored) {
            return 0;
        }
    }

    private String formatResultText(
            String state,
            String message,
            String ssid,
            String ip,
            String wifiIp,
            String wiredIp,
            String wifiInterface,
            String errorCode,
            String applyOutput,
            String diagnosticHint,
            String wpaUnit,
            int ipWaitSeconds,
            String rawJson
    ) {
        StringBuilder builder = new StringBuilder("Result:");
~~~

## `apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java` (3/3)

- bytes: 92720
- segment: 3/3

~~~java
        appendResultLine(builder, "State", state);
        appendResultLine(builder, "Message", message);
        appendResultLine(builder, "SSID", ssid);
        appendResultLine(builder, "IP", ip);
        appendResultLine(builder, "Wi-Fi IP", wifiIp);
        appendResultLine(builder, "Wired IP", wiredIp);
        appendResultLine(builder, "Wi-Fi interface", wifiInterface);
        appendResultLine(builder, "WPA unit", wpaUnit);
        appendResultLine(builder, "Error code", errorCode);
        appendResultLine(builder, "Apply output", applyOutput);
        appendResultLine(builder, "Diagnostic", diagnosticHint);
        if (ipWaitSeconds > 0) {
            appendResultLine(builder, "DHCP wait", ipWaitSeconds + " seconds");
        }
        appendResultLine(builder, "Raw", rawJson);
        return builder.toString();
    }

    private void appendResultLine(StringBuilder builder, String label, String value) {
        if (value == null || value.isEmpty()) {
            return;
        }
        builder.append("\n").append(label).append(": ").append(value);
    }

    private void openWebUi() {
        openInAppUrl(webUrl);
    }

    private void openRelativeUrl(String suffix) {
        if (webUrl == null || webUrl.isEmpty()) {
            setStatus("No WebUI URL reported yet.");
            return;
        }
        String normalized = webUrl.endsWith("/") ? webUrl.substring(0, webUrl.length() - 1) : webUrl;
        openInAppUrl(normalized + suffix);
    }

    @SuppressLint("MissingPermission")
    private void disconnectFromDevice() {
        if (connectionMode == ConnectionMode.LUMELO) {
            if (classicTransport == null || !classicTransport.isConnected()) {
                setStatus("No active classic Bluetooth connection to disconnect.");
                return;
            }
            stopStatusPolling();
            classicTransport.disconnect();
            resetProvisioningSession();
            setStatus("Disconnected from T4.");
            logEvent("Disconnected from T4");
            return;
        }
        if (gatt == null) {
            setStatus("No active BLE connection to disconnect.");
            return;
        }
        ConnectionMode disconnectedMode = connectionMode;
        stopStatusPolling();
        closeGattQuietly(true);
        resetProvisioningSession();
        setStatus(disconnectedMode == ConnectionMode.LUMELO
                ? "Disconnected from T4."
                : "Disconnected from raw BLE device.");
        logEvent(disconnectedMode == ConnectionMode.LUMELO
                ? "Disconnected from T4"
                : "Disconnected from raw BLE device");
    }

    @SuppressLint("MissingPermission")
    private void closeGattQuietly(boolean requestDisconnect) {
        BluetoothGatt currentGatt = gatt;
        gatt = null;
        if (currentGatt == null) {
            return;
        }
        if (requestDisconnect && hasConnectPermission()) {
            currentGatt.disconnect();
        }
        currentGatt.close();
    }

    private void finishGattSession(BluetoothGatt bluetoothGatt) {
        if (gatt == bluetoothGatt) {
            gatt = null;
        }
        bluetoothGatt.close();
        resetProvisioningSession();
    }

    private void resetProvisioningSession() {
        stopStatusPolling();
        deviceInfoCharacteristic = null;
        wifiCredentialsCharacteristic = null;
        applyCharacteristic = null;
        statusCharacteristic = null;
        webUrl = null;
        mainInterfaceOpenedForSession = false;
        writeQueue.clear();
        writeInFlight = false;
        connectionMode = ConnectionMode.LUMELO;
        negotiatedMtu = DEFAULT_ATT_MTU;
        serviceDiscoveryStarted = false;
        runOnUiThread(() -> {
            sendButton.setEnabled(false);
            readStatusButton.setEnabled(false);
            disconnectButton.setEnabled(false);
            setWebButtonsEnabled(webUrl != null && !webUrl.isEmpty());
            deviceInfoView.setText("Device info: not connected");
            resultView.setText("Result: waiting");
        });
    }

    private boolean maybeOpenMainInterface(String state) {
        if (!"connected".equals(state) || mainInterfaceOpenedForSession || webUrl == null || webUrl.isEmpty()) {
            return false;
        }
        if (!shouldAutoOpenWebUi()) {
            return false;
        }
        mainInterfaceOpenedForSession = true;
        logEvent("Opening Lumelo main interface");
        openInAppUrl(webUrl);
        return true;
    }

    private void openInAppUrl(String url) {
        if (url == null || url.isEmpty()) {
            setStatus("No WebUI URL reported yet.");
            return;
        }
        persistWebUiUrl(url);
        Intent intent = new Intent(this, MainInterfaceActivity.class);
        intent.putExtra(MainInterfaceActivity.EXTRA_INITIAL_URL, url);
        startActivity(intent);
    }

    private void setWebButtonsEnabled(boolean enabled) {
        openWebButton.setEnabled(enabled);
        openProvisioningButton.setEnabled(enabled);
        openLogsButton.setEnabled(enabled);
        openHealthzButton.setEnabled(enabled);
    }

    private void persistWebUiUrl(String url) {
        if (url == null || url.isEmpty()) {
            return;
        }
        prefs().edit().putString(PREF_LAST_WEB_URL, url).commit();
    }

    @SuppressLint("MissingPermission")
    private void rememberClassicDevice(BluetoothDevice device) {
        if (device == null) {
            return;
        }
        rememberClassicDevice(device.getAddress(), sanitize(device.getName()));
    }

    private void rememberClassicDevice(String address, String preferredName) {
        if (address == null || address.isEmpty() || !BluetoothAdapter.checkBluetoothAddress(address)) {
            return;
        }
        String effectiveName = sanitize(preferredName);
        if (effectiveName.isEmpty()) {
            effectiveName = "Lumelo T4";
        }
        prefs().edit()
                .putString(PREF_LAST_CLASSIC_ADDRESS, address)
                .putString(PREF_LAST_CLASSIC_NAME, effectiveName)
                .commit();
    }

    private String rememberedClassicName(String address) {
        if (address == null || address.isEmpty()) {
            return "";
        }
        String rememberedAddress = prefs().getString(PREF_LAST_CLASSIC_ADDRESS, "");
        if (rememberedAddress == null || !rememberedAddress.equalsIgnoreCase(address)) {
            return "";
        }
        return sanitize(prefs().getString(PREF_LAST_CLASSIC_NAME, ""));
    }

    private void restorePersistedWebUiUrl() {
        if (webUrl != null && !webUrl.isEmpty()) {
            setWebButtonsEnabled(true);
            return;
        }
        String rememberedUrl = prefs().getString(PREF_LAST_WEB_URL, "");
        if (rememberedUrl == null || rememberedUrl.isEmpty()) {
            return;
        }
        webUrl = rememberedUrl;
        runOnUiThread(() -> {
            setWebButtonsEnabled(true);
            if (!disconnectButton.isEnabled()) {
                resultView.setText(
                        "Result:\nLast known WebUI: " + rememberedUrl
                                + "\nTip: if phone and T4 are on the same network, you can open it directly."
                );
            }
        });
    }

    private SharedPreferences prefs() {
        return getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
    }

    private boolean shouldAutoOpenWebUi() {
        String targetHost = extractHost(webUrl);
        if (targetHost.isEmpty() || !isPrivateIpv4(targetHost)) {
            return true;
        }
        String phoneIp = currentIpv4Address();
        if (phoneIp.isEmpty()) {
            return false;
        }
        return sameSubnet(phoneIp, targetHost);
    }

    private String crossNetworkHint() {
        String targetHost = extractHost(webUrl);
        String phoneIp = currentIpv4Address();
        if (targetHost.isEmpty() || phoneIp.isEmpty()) {
            return "";
        }
        if (!isPrivateIpv4(targetHost) || sameSubnet(phoneIp, targetHost)) {
            return "";
        }
        return "Phone IP " + phoneIp + " is not on the same local subnet as T4 " + targetHost
                + ". Connect this phone to the same hotspot or router before opening WebUI.";
    }

    private String currentIpv4Address() {
        ConnectivityManager manager = getSystemService(ConnectivityManager.class);
        if (manager == null) {
            return "";
        }
        Network activeNetwork = manager.getActiveNetwork();
        if (activeNetwork == null) {
            return "";
        }
        LinkProperties properties = manager.getLinkProperties(activeNetwork);
        if (properties == null) {
            return "";
        }
        List<LinkAddress> addresses = properties.getLinkAddresses();
        if (addresses == null) {
            return "";
        }
        for (LinkAddress address : addresses) {
            InetAddress inetAddress = address.getAddress();
            if (inetAddress instanceof Inet4Address && !inetAddress.isLoopbackAddress()) {
                return inetAddress.getHostAddress();
            }
        }
        return "";
    }

    private String extractHost(String url) {
        if (url == null || url.isEmpty()) {
            return "";
        }
        int schemeIndex = url.indexOf("://");
        String remainder = schemeIndex >= 0 ? url.substring(schemeIndex + 3) : url;
        int slashIndex = remainder.indexOf('/');
        if (slashIndex >= 0) {
            remainder = remainder.substring(0, slashIndex);
        }
        int colonIndex = remainder.indexOf(':');
        if (colonIndex >= 0) {
            remainder = remainder.substring(0, colonIndex);
        }
        return remainder;
    }

    private boolean isPrivateIpv4(String ip) {
        if (ip.startsWith("10.")) {
            return true;
        }
        if (ip.startsWith("192.168.")) {
            return true;
        }
        if (!ip.startsWith("172.")) {
            return false;
        }
        String[] parts = ip.split("\\.");
        if (parts.length < 2) {
            return false;
        }
        try {
            int secondOctet = Integer.parseInt(parts[1]);
            return secondOctet >= 16 && secondOctet <= 31;
        } catch (NumberFormatException ignored) {
            return false;
        }
    }

    private boolean sameSubnet(String left, String right) {
        String[] leftParts = left.split("\\.");
        String[] rightParts = right.split("\\.");
        if (leftParts.length != 4 || rightParts.length != 4) {
            return false;
        }
        return leftParts[0].equals(rightParts[0])
                && leftParts[1].equals(rightParts[1])
                && leftParts[2].equals(rightParts[2]);
    }

    private void startStatusPolling() {
        statusPollingDeadlineMs = System.currentTimeMillis() + STATUS_POLL_TIMEOUT_MS;
        if (statusPollingActive) {
            return;
        }
        statusPollingActive = true;
        logEvent("Automatic status polling started");
        handler.postDelayed(statusPollingRunnable, STATUS_POLL_INTERVAL_MS);
    }

    private void stopStatusPolling() {
        if (!statusPollingActive) {
            return;
        }
        statusPollingActive = false;
        handler.removeCallbacks(statusPollingRunnable);
        logEvent("Automatic status polling stopped");
    }

    @SuppressLint("MissingPermission")
    private boolean requestPreferredMtu(BluetoothGatt bluetoothGatt) {
        if (bluetoothGatt == null || Build.VERSION.SDK_INT < Build.VERSION_CODES.LOLLIPOP) {
            return false;
        }
        try {
            return bluetoothGatt.requestMtu(DESIRED_ATT_MTU);
        } catch (SecurityException ignored) {
            return false;
        }
    }

    @SuppressLint("MissingPermission")
    private void startServiceDiscovery(BluetoothGatt bluetoothGatt) {
        if (bluetoothGatt == null || serviceDiscoveryStarted) {
            return;
        }
        serviceDiscoveryStarted = true;
        if (!bluetoothGatt.discoverServices()) {
            serviceDiscoveryStarted = false;
            setStatus(connectionMode == ConnectionMode.LUMELO
                    ? "Lumelo service discovery could not start."
                    : "BLE service discovery could not start.");
            logEvent("Service discovery could not start");
        } else {
            logEvent("Service discovery started");
        }
    }

    private String currentConnectedWifiSsid() {
        if (checkSelfPermission(Manifest.permission.ACCESS_WIFI_STATE) != PackageManager.PERMISSION_GRANTED) {
            return "";
        }
        WifiManager wifiManager = (WifiManager) getApplicationContext().getSystemService(Context.WIFI_SERVICE);
        if (wifiManager == null) {
            return "";
        }
        WifiInfo wifiInfo = wifiManager.getConnectionInfo();
        if (wifiInfo == null) {
            return "";
        }
        String ssid = wifiInfo.getSSID();
        if (ssid == null || ssid.isEmpty() || "<unknown ssid>".equalsIgnoreCase(ssid)) {
            return "";
        }
        if (ssid.length() >= 2 && ssid.startsWith("\"") && ssid.endsWith("\"")) {
            return ssid.substring(1, ssid.length() - 1);
        }
        return ssid;
    }

    private String describeGattStatus(int status) {
        switch (status) {
            case BluetoothGatt.GATT_SUCCESS:
                return "GATT_SUCCESS(0)";
            case BluetoothGatt.GATT_READ_NOT_PERMITTED:
                return "GATT_READ_NOT_PERMITTED(" + status + ")";
            case BluetoothGatt.GATT_WRITE_NOT_PERMITTED:
                return "GATT_WRITE_NOT_PERMITTED(" + status + ")";
            case BluetoothGatt.GATT_INSUFFICIENT_AUTHENTICATION:
                return "GATT_INSUFFICIENT_AUTHENTICATION(" + status + ")";
            case BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED:
                return "GATT_REQUEST_NOT_SUPPORTED(" + status + ")";
            case BluetoothGatt.GATT_INSUFFICIENT_ENCRYPTION:
                return "GATT_INSUFFICIENT_ENCRYPTION(" + status + ")";
            case BluetoothGatt.GATT_INVALID_OFFSET:
                return "GATT_INVALID_OFFSET(" + status + ")";
            case BluetoothGatt.GATT_INVALID_ATTRIBUTE_LENGTH:
                return "GATT_INVALID_ATTRIBUTE_LENGTH(" + status + ")";
            case BluetoothGatt.GATT_CONNECTION_CONGESTED:
                return "GATT_CONNECTION_CONGESTED(" + status + ")";
            default:
                return "status=" + status;
        }
    }

    private String describeConnectionState(int state) {
        switch (state) {
            case BluetoothProfile.STATE_DISCONNECTED:
                return "DISCONNECTED";
            case BluetoothProfile.STATE_CONNECTING:
                return "CONNECTING";
            case BluetoothProfile.STATE_CONNECTED:
                return "CONNECTED";
            case BluetoothProfile.STATE_DISCONNECTING:
                return "DISCONNECTING";
            default:
                return "state=" + state;
        }
    }

    private static final class WriteRequest {
        final BluetoothGattCharacteristic characteristic;
        final byte[] value;

        WriteRequest(BluetoothGattCharacteristic characteristic, byte[] value) {
            this.characteristic = characteristic;
            this.value = value;
        }
    }

    private static final class ObservedDevice {
        final BluetoothDevice device;
        final ScanObservation observation;

        ObservedDevice(BluetoothDevice device, ScanObservation observation) {
            this.device = device;
            this.observation = observation;
        }
    }

    private static final class ScanObservation {
        final String address;
        final boolean uuidMatch;
        final boolean nameMatch;
        final boolean bonded;
        final boolean remembered;
        final String listEntry;
        final String detailText;
        final String logLine;

        ScanObservation(
                String address,
                boolean uuidMatch,
                boolean nameMatch,
                boolean bonded,
                boolean remembered,
                String listEntry,
                String detailText,
                String logLine
        ) {
            this.address = address;
            this.uuidMatch = uuidMatch;
            this.nameMatch = nameMatch;
            this.bonded = bonded;
            this.remembered = remembered;
            this.listEntry = listEntry;
            this.detailText = detailText;
            this.logLine = logLine;
        }
    }
}
~~~

