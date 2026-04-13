# AI Review Part 02

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainActivity.java` (1/3)

- bytes: 92720
- segment: 1/3

~~~java
package com.lumelo.provisioning;

import android.Manifest;
import android.annotation.SuppressLint;
import android.app.Activity;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothDevice;
import android.bluetooth.BluetoothGatt;
import android.bluetooth.BluetoothGattCallback;
import android.bluetooth.BluetoothGattCharacteristic;
import android.bluetooth.BluetoothGattDescriptor;
import android.bluetooth.BluetoothGattService;
import android.bluetooth.BluetoothManager;
import android.bluetooth.BluetoothProfile;
import android.bluetooth.le.BluetoothLeScanner;
import android.bluetooth.le.ScanRecord;
import android.bluetooth.le.ScanCallback;
import android.bluetooth.le.ScanResult;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.net.ConnectivityManager;
import android.net.LinkAddress;
import android.net.LinkProperties;
import android.net.Network;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.util.SparseArray;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ListView;
import android.widget.ScrollView;
import android.widget.TextView;

import java.nio.charset.StandardCharsets;
import java.net.Inet4Address;
import java.net.InetAddress;
import java.text.SimpleDateFormat;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.Date;
import java.util.List;
import java.util.Locale;
import java.util.UUID;

public class MainActivity extends Activity {
    private static final String PREFS_NAME = "lumelo_setup";
    private static final String PREF_LAST_WEB_URL = "last_web_url";
    private static final String PREF_LAST_CLASSIC_ADDRESS = "last_classic_address";
    private static final String PREF_LAST_CLASSIC_NAME = "last_classic_name";
    private enum ScanMode {
        LUMELO,
        GENERIC_TEST
    }

    private enum ConnectionMode {
        LUMELO,
        GENERIC_TEST
    }

    private static final int REQUEST_BLE_PERMISSIONS = 7001;
    private static final int DEFAULT_ATT_MTU = 23;
    private static final int DESIRED_ATT_MTU = 517;
    private static final int DEBUG_LOG_HISTORY_LIMIT = 120;
    private static final int DEBUG_LOG_VIEW_LIMIT = 16;
    private static final long MTU_REQUEST_TIMEOUT_MS = 3_000;
    private static final long SCAN_WINDOW_MS = 12_000;
    private static final long STATUS_POLL_INTERVAL_MS = 3_000;
    private static final long STATUS_POLL_TIMEOUT_MS = 60_000;

    private static final UUID SERVICE_UUID = UUID.fromString("7b6a0001-8d8b-4e31-9f0a-9a9f4d1f2c10");
    private static final UUID DEVICE_INFO_UUID = UUID.fromString("7b6a0002-8d8b-4e31-9f0a-9a9f4d1f2c10");
    private static final UUID WIFI_CREDENTIALS_UUID = UUID.fromString("7b6a0003-8d8b-4e31-9f0a-9a9f4d1f2c10");
    private static final UUID APPLY_UUID = UUID.fromString("7b6a0004-8d8b-4e31-9f0a-9a9f4d1f2c10");
    private static final UUID STATUS_UUID = UUID.fromString("7b6a0005-8d8b-4e31-9f0a-9a9f4d1f2c10");
    private static final UUID CLIENT_CONFIG_UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb");

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final List<BluetoothDevice> devices = new ArrayList<>();
    private final List<ScanObservation> scanObservations = new ArrayList<>();
    private final ArrayDeque<WriteRequest> writeQueue = new ArrayDeque<>();

    private ArrayAdapter<String> deviceAdapter;
    private BluetoothAdapter bluetoothAdapter;
    private BluetoothLeScanner scanner;
    private ClassicBluetoothTransport classicTransport;
    private BluetoothDevice selectedDevice;
    private BluetoothGatt gatt;
    private BluetoothGattCharacteristic deviceInfoCharacteristic;
    private BluetoothGattCharacteristic wifiCredentialsCharacteristic;
    private BluetoothGattCharacteristic applyCharacteristic;
    private BluetoothGattCharacteristic statusCharacteristic;
    private boolean scanning;
    private boolean classicDiscoveryReceiverRegistered;
    private boolean writeInFlight;
    private boolean serviceDiscoveryStarted;
    private ScanMode activeScanMode = ScanMode.LUMELO;
    private ConnectionMode connectionMode = ConnectionMode.LUMELO;
    private int negotiatedMtu = DEFAULT_ATT_MTU;

    private TextView statusView;
    private TextView buildInfoView;
    private TextView environmentView;
    private TextView scanSummaryView;
    private TextView selectedView;
    private TextView deviceInfoView;
    private TextView resultView;
    private TextView debugLogView;
    private Button scanButton;
    private Button testScanButton;
    private Button connectButton;
    private Button useCurrentWifiButton;
    private Button sendButton;
    private Button readStatusButton;
    private Button disconnectButton;
    private Button openWebButton;
    private Button openProvisioningButton;
    private Button openLogsButton;
    private Button openHealthzButton;
    private Button clearLogButton;
    private Button exportLogButton;
    private EditText ssidInput;
    private EditText passwordInput;
    private String webUrl;
    private boolean mainInterfaceOpenedForSession;
    private long statusPollingDeadlineMs;
    private boolean statusPollingActive;
    private final ArrayDeque<String> debugLines = new ArrayDeque<>();
    private String lastLoggedMessage = "";
    private final Runnable statusPollingRunnable = new Runnable() {
        @Override
        public void run() {
            if (!statusPollingActive) {
                return;
            }
            if (System.currentTimeMillis() >= statusPollingDeadlineMs) {
                statusPollingActive = false;
                logEvent("Automatic status polling timed out");
                return;
            }
            if (connectionMode == ConnectionMode.LUMELO && classicTransport != null && classicTransport.isConnected()) {
                requestStatusRead();
            } else if (gatt != null && statusCharacteristic != null && hasConnectPermission()) {
                requestStatusRead(gatt);
            }
            handler.postDelayed(this, STATUS_POLL_INTERVAL_MS);
        }
    };
    private final BroadcastReceiver classicDiscoveryReceiver = new BroadcastReceiver() {
        @SuppressLint("MissingPermission")
        @Override
        public void onReceive(Context context, Intent intent) {
            String action = intent.getAction();
            if (BluetoothDevice.ACTION_FOUND.equals(action)
                    || BluetoothDevice.ACTION_NAME_CHANGED.equals(action)) {
                BluetoothDevice device = intent.getParcelableExtra(BluetoothDevice.EXTRA_DEVICE);
                if (device != null) {
                    handleClassicScanResult(
                            device,
                            sanitize(intent.getStringExtra(BluetoothDevice.EXTRA_NAME))
                    );
                }
                return;
            }
            if (BluetoothAdapter.ACTION_DISCOVERY_FINISHED.equals(action)) {
                stopScan();
            }
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setTitle("Lumelo Setup");
        buildUi();
        setupBluetooth();
        requestBlePermissionsIfNeeded();
        restorePersistedWebUiUrl();
        refreshEnvironmentStatus();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        stopScan();
        stopStatusPolling();
        unregisterClassicDiscoveryReceiver();
        if (classicTransport != null) {
            classicTransport.disconnect();
        }
        closeGattQuietly(true);
    }

    @Override
    protected void onResume() {
        super.onResume();
        restorePersistedWebUiUrl();
        refreshEnvironmentStatus();
    }

    private void buildUi() {
        int padding = dp(18);
        ScrollView scrollView = new ScrollView(this);
        scrollView.setFillViewport(true);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(padding, padding, padding, padding);
        root.setBackgroundColor(0xfff3efe7);
        scrollView.addView(root, new ScrollView.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
        ));
        setContentView(scrollView);

        TextView title = new TextView(this);
        title.setText("Lumelo Wi-Fi Setup");
        title.setTextSize(26);
        title.setTextColor(0xff1f1d1a);
        root.addView(title, matchWrap());

        buildInfoView = label(buildBuildInfoText());
        root.addView(buildInfoView, matchWrap());

        statusView = label("Ready. Turn on the T4 and scan for Lumelo T4.");
        root.addView(statusView, matchWrap());

        environmentView = label("Environment: checking Bluetooth and permissions...");
        root.addView(environmentView, matchWrap());

        scanSummaryView = label("Scan summary: not started");
        root.addView(scanSummaryView, matchWrap());

        scanButton = button("Scan for Lumelo");
        scanButton.setOnClickListener(view -> startScan(ScanMode.LUMELO));
        root.addView(scanButton, matchWrap());

        testScanButton = button("Raw BLE Scan");
        testScanButton.setOnClickListener(view -> startScan(ScanMode.GENERIC_TEST));
        root.addView(testScanButton, matchWrap());

        ListView listView = new ListView(this);
        deviceAdapter = new ArrayAdapter<>(this, android.R.layout.simple_list_item_1, new ArrayList<>());
        listView.setAdapter(deviceAdapter);
        listView.setOnItemClickListener((parent, view, position, id) -> {
            setSelectedScanDevice(devices.get(position), scanObservations.get(position));
            refreshScanSummary();
        });
        root.addView(listView, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(220)));

        selectedView = label("Selected: none");
        root.addView(selectedView, matchWrap());

        connectButton = button("Connect");
        connectButton.setEnabled(false);
        connectButton.setOnClickListener(view -> connectSelectedDevice());
        root.addView(connectButton, matchWrap());

        deviceInfoView = label("Device info: not connected");
        root.addView(deviceInfoView, matchWrap());

        ssidInput = input("Wi-Fi SSID", false);
        root.addView(ssidInput, matchWrap());

        passwordInput = input("Wi-Fi password", true);
        root.addView(passwordInput, matchWrap());

        useCurrentWifiButton = button("Use Current Wi-Fi");
        useCurrentWifiButton.setOnClickListener(view -> fillCurrentWifiSsid());
        root.addView(useCurrentWifiButton, matchWrap());

        sendButton = button("Send Wi-Fi Credentials");
        sendButton.setEnabled(false);
        sendButton.setOnClickListener(view -> sendWifiCredentials());
        root.addView(sendButton, matchWrap());

        readStatusButton = button("Read Status");
        readStatusButton.setEnabled(false);
        readStatusButton.setOnClickListener(view -> requestStatusRead());
        root.addView(readStatusButton, matchWrap());

        disconnectButton = button("Disconnect");
        disconnectButton.setEnabled(false);
        disconnectButton.setOnClickListener(view -> disconnectFromDevice());
        root.addView(disconnectButton, matchWrap());

        resultView = label("Result: waiting");
        root.addView(resultView, matchWrap());

        openWebButton = button("Open WebUI");
        openWebButton.setEnabled(false);
        openWebButton.setOnClickListener(view -> openWebUi());
        root.addView(openWebButton, matchWrap());

        openProvisioningButton = button("Open Provisioning");
        openProvisioningButton.setEnabled(false);
        openProvisioningButton.setOnClickListener(view -> openRelativeUrl("/provisioning"));
        root.addView(openProvisioningButton, matchWrap());

        openLogsButton = button("Open Logs");
        openLogsButton.setEnabled(false);
        openLogsButton.setOnClickListener(view -> openRelativeUrl("/logs"));
        root.addView(openLogsButton, matchWrap());

        openHealthzButton = button("Open Healthz");
        openHealthzButton.setEnabled(false);
        openHealthzButton.setOnClickListener(view -> openRelativeUrl("/healthz"));
        root.addView(openHealthzButton, matchWrap());

        clearLogButton = button("Clear Debug Log");
        clearLogButton.setOnClickListener(view -> clearDebugLog());
        root.addView(clearLogButton, matchWrap());

        exportLogButton = button("Export Diagnostics");
        exportLogButton.setOnClickListener(view -> exportDiagnostics());
        root.addView(exportLogButton, matchWrap());

        debugLogView = label("Debug log:\n- app ready");
        root.addView(debugLogView, matchWrap());
    }

    private void setupBluetooth() {
        BluetoothManager manager = (BluetoothManager) getSystemService(Context.BLUETOOTH_SERVICE);
        if (manager == null) {
            setStatus("Bluetooth manager unavailable.");
            logEvent("Bluetooth manager unavailable");
            return;
        }
        bluetoothAdapter = manager.getAdapter();
        if (bluetoothAdapter == null) {
            setStatus("This phone has no Bluetooth adapter.");
            logEvent("This phone has no Bluetooth adapter");
            return;
        }
        scanner = bluetoothAdapter.getBluetoothLeScanner();
        classicTransport = new ClassicBluetoothTransport(new ClassicBluetoothTransport.Listener() {
            @Override
            public void onClassicConnected(BluetoothDevice device) {
                rememberClassicDevice(device);
                runOnUiThread(() -> {
                    sendButton.setEnabled(true);
                    readStatusButton.setEnabled(true);
                    disconnectButton.setEnabled(true);
                });
                setStatus("Classic Bluetooth connected. Reading T4 info...");
                logEvent("Classic Bluetooth connected to " + displayName(device));
            }

            @Override
            public void onClassicDisconnected(String message) {
                stopStatusPolling();
                resetProvisioningSession();
                setStatus(message);
                logEvent(message);
            }

            @Override
            public void onClassicDeviceInfo(String payload) {
                runOnUiThread(() -> deviceInfoView.setText("Device info: " + payload));
                logEvent("Classic device info payload: " + payload);
            }

            @Override
            public void onClassicStatus(String payload) {
                showResult(payload);
            }

            @Override
            public void onClassicError(String message) {
                setStatus(message);
                logEvent(message);
            }

            @Override
            public void onClassicLog(String message) {
                logEvent(message);
            }
        });
        logEvent("Bluetooth adapter initialized");
    }

    private void requestBlePermissionsIfNeeded() {
        if (hasScanPermission() && hasConnectPermission()) {
            setStatus("Permissions ready. Scan for Lumelo T4.");
            refreshEnvironmentStatus();
            return;
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            requestPermissions(
                    new String[]{
                            Manifest.permission.BLUETOOTH_SCAN,
                            Manifest.permission.BLUETOOTH_CONNECT,
                            Manifest.permission.ACCESS_FINE_LOCATION,
                    },
                    REQUEST_BLE_PERMISSIONS
            );
        } else {
            requestPermissions(
                    new String[]{Manifest.permission.ACCESS_FINE_LOCATION},
                    REQUEST_BLE_PERMISSIONS
            );
        }
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode != REQUEST_BLE_PERMISSIONS) {
            return;
        }
        if (hasScanPermission() && hasConnectPermission()) {
            setStatus("Permissions ready. Scan for Lumelo T4.");
            logEvent("BLE permissions granted");
        } else {
            setStatus("Bluetooth permissions are required for provisioning.");
            logEvent("BLE permissions denied or incomplete");
        }
        refreshEnvironmentStatus();
    }

    @SuppressLint("MissingPermission")
    private void startScan(ScanMode scanMode) {
        if (!hasScanPermission() || !hasConnectPermission()) {
            logEvent("Scan blocked: missing Bluetooth permissions");
            requestBlePermissionsIfNeeded();
            return;
        }
        if (bluetoothAdapter == null) {
            setStatus("Bluetooth adapter unavailable.");
            logEvent("Scan blocked: Bluetooth adapter unavailable");
            return;
        }
        if (!bluetoothAdapter.isEnabled()) {
            logEvent("Scan blocked: Bluetooth is disabled");
            startActivity(new Intent(BluetoothAdapter.ACTION_REQUEST_ENABLE));
            return;
        }

        activeScanMode = scanMode;
        devices.clear();
        scanObservations.clear();
        deviceAdapter.clear();
        selectedDevice = null;
        selectedView.setText("Selected: none");
        connectButton.setEnabled(false);
        refreshScanSummary();

        scanning = true;
        scanButton.setEnabled(false);
        testScanButton.setEnabled(false);

        if (scanMode == ScanMode.LUMELO) {
            startClassicScan();
            return;
        }

        scanner = bluetoothAdapter.getBluetoothLeScanner();
        if (scanner == null) {
            setStatus("BLE scanner unavailable.");
            logEvent("Scan blocked: BLE scanner unavailable");
            scanning = false;
            scanButton.setEnabled(true);
            testScanButton.setEnabled(true);
            return;
        }
        setStatus(scanMode == ScanMode.LUMELO
                ? "Scanning for Lumelo T4..."
                : "Scanning raw nearby BLE advertisements...");
        logEvent("Raw BLE scan started");
        scanner.startScan(scanCallback);
        handler.postDelayed(this::stopScan, SCAN_WINDOW_MS);
    }

    @SuppressLint("MissingPermission")
    private void stopScan() {
        if (!scanning) {
            return;
        }
        scanning = false;
        scanButton.setEnabled(true);
        testScanButton.setEnabled(true);

        if (activeScanMode == ScanMode.LUMELO) {
            stopClassicScan();
            int matchedDevices = countClassicNameMatches();
            if (devices.isEmpty()) {
                setStatus("Scan finished. No Lumelo device found.");
                logEvent("Classic Bluetooth scan finished: no Lumelo device found");
            } else if (matchedDevices == 0) {
                setStatus("Scan finished. No named Lumelo device found. Showing classic candidates.");
                logEvent("Classic Bluetooth scan finished: no named Lumelo match; showing "
                        + devices.size() + " classic candidate(s)");
            } else {
                setStatus("Scan finished.");
                logEvent("Classic Bluetooth scan finished: found " + matchedDevices
                        + " named Lumelo candidate(s)");
            }
            refreshScanSummary();
            return;
        }

        if (scanner == null || !hasScanPermission() || !hasConnectPermission()) {
            return;
        }
        scanner.stopScan(scanCallback);
        setStatus(devices.isEmpty() ? "Scan finished. No nearby BLE device found." : "Scan finished.");
        logEvent(devices.isEmpty()
                ? "Raw BLE scan finished: no nearby BLE device found"
                : "Raw BLE scan finished: found " + devices.size() + " BLE device(s)");
        refreshScanSummary();
    }

    @SuppressLint("MissingPermission")
    private void startClassicScan() {
        registerClassicDiscoveryReceiver();
        seedBondedLumeloDevices();
        seedRememberedLumeloDevice();
        if (bluetoothAdapter.isDiscovering()) {
            bluetoothAdapter.cancelDiscovery();
        }
        setStatus("Scanning for Lumelo T4 over classic Bluetooth...");
        logEvent("Classic Bluetooth Lumelo scan started");
        if (!bluetoothAdapter.startDiscovery()) {
            scanning = false;
            scanButton.setEnabled(true);
            testScanButton.setEnabled(true);
            if (!devices.isEmpty()) {
                setStatus("Classic Bluetooth discovery could not start. Showing paired Lumelo devices.");
                logEvent("Classic Bluetooth discovery could not start; using paired Lumelo devices");
                refreshScanSummary();
                return;
            }
            setStatus("Classic Bluetooth discovery could not start.");
            logEvent("Classic Bluetooth discovery could not start");
            return;
        }
        handler.postDelayed(this::stopScan, SCAN_WINDOW_MS);
    }

    @SuppressLint("MissingPermission")
    private void stopClassicScan() {
        if (bluetoothAdapter != null && bluetoothAdapter.isDiscovering()) {
            bluetoothAdapter.cancelDiscovery();
        }
        unregisterClassicDiscoveryReceiver();
    }

    @SuppressLint("MissingPermission")
    private void seedBondedLumeloDevices() {
        if (bluetoothAdapter == null || !hasConnectPermission()) {
            return;
        }
        for (BluetoothDevice bondedDevice : bluetoothAdapter.getBondedDevices()) {
            ScanObservation observation = describeClassicObservation(bondedDevice, "");
            if (observation == null || !observation.nameMatch) {
                continue;
            }
            upsertClassicObservation(bondedDevice, observation);
        }
    }

    @SuppressLint("MissingPermission")
    private void seedRememberedLumeloDevice() {
        if (bluetoothAdapter == null || !hasConnectPermission()) {
            return;
        }
        String rememberedAddress = prefs().getString(PREF_LAST_CLASSIC_ADDRESS, "");
        if (rememberedAddress == null || rememberedAddress.isEmpty()) {
            return;
        }
        if (!BluetoothAdapter.checkBluetoothAddress(rememberedAddress)) {
            return;
        }
        String rememberedName = sanitize(prefs().getString(PREF_LAST_CLASSIC_NAME, ""));
        if (rememberedName.isEmpty()) {
            rememberedName = "Lumelo T4";
        }
        try {
            BluetoothDevice rememberedDevice = bluetoothAdapter.getRemoteDevice(rememberedAddress);
            ScanObservation observation = describeRememberedClassicObservation(rememberedDevice, rememberedName);
            if (observation != null) {
                upsertClassicObservation(rememberedDevice, observation);
            }
        } catch (IllegalArgumentException ignored) {
            // Ignore invalid persisted addresses from older debug builds.
        }
    }

    private void registerClassicDiscoveryReceiver() {
        if (classicDiscoveryReceiverRegistered) {
            return;
        }
        IntentFilter filter = new IntentFilter();
        filter.addAction(BluetoothDevice.ACTION_FOUND);
        filter.addAction(BluetoothDevice.ACTION_NAME_CHANGED);
        filter.addAction(BluetoothAdapter.ACTION_DISCOVERY_FINISHED);
        registerReceiver(classicDiscoveryReceiver, filter);
        classicDiscoveryReceiverRegistered = true;
    }

    private void unregisterClassicDiscoveryReceiver() {
        if (!classicDiscoveryReceiverRegistered) {
            return;
        }
        unregisterReceiver(classicDiscoveryReceiver);
        classicDiscoveryReceiverRegistered = false;
    }

    private final ScanCallback scanCallback = new ScanCallback() {
        @Override
        public void onScanResult(int callbackType, ScanResult result) {
            BluetoothDevice device = result.getDevice();
            ScanObservation observation = describeScanObservation(result);
            if (device == null || observation == null || !shouldIncludeScanResult(observation)) {
                return;
            }
            String address = device.getAddress();
            for (int index = 0; index < scanObservations.size(); index++) {
                if (address.equals(scanObservations.get(index).address)) {
                    return;
                }
            }
            devices.add(device);
            scanObservations.add(observation);
            deviceAdapter.add(observation.listEntry);
            refreshScanSummary();
            logEvent(observation.logLine);
        }
    };

    @SuppressLint("MissingPermission")
    private void handleClassicScanResult(BluetoothDevice device, String discoveredName) {
        ScanObservation observation = describeClassicObservation(device, discoveredName);
        if (device == null || observation == null) {
            return;
        }
        if (observation.nameMatch) {
            rememberClassicDevice(device.getAddress(), resolveClassicName(device, discoveredName));
        }
        upsertClassicObservation(device, observation);
    }

    private void upsertClassicObservation(BluetoothDevice device, ScanObservation observation) {
        String address = device.getAddress();
        for (int index = 0; index < scanObservations.size(); index++) {
            if (!address.equals(scanObservations.get(index).address)) {
                continue;
            }
            ScanObservation existing = scanObservations.get(index);
            boolean changed = existing.nameMatch != observation.nameMatch
                    || existing.bonded != observation.bonded
                    || existing.remembered != observation.remembered
                    || !existing.listEntry.equals(observation.listEntry)
                    || !existing.logLine.equals(observation.logLine);
            if (!changed) {
                return;
            }
            scanObservations.set(index, observation);
            devices.set(index, device);
            sortScanResults();
            rebuildDeviceAdapter();
            refreshSelectionState();
            refreshScanSummary();
            logEvent(observation.logLine);
            return;
        }

        devices.add(device);
        scanObservations.add(observation);
        sortScanResults();
        rebuildDeviceAdapter();
        refreshSelectionState();
        refreshScanSummary();
        logEvent(observation.logLine);
    }

    private void rebuildDeviceAdapter() {
        deviceAdapter.clear();
        for (ScanObservation observation : scanObservations) {
            deviceAdapter.add(observation.listEntry);
        }
    }

    private void sortScanResults() {
        List<ObservedDevice> ordered = new ArrayList<>();
        for (int index = 0; index < scanObservations.size(); index++) {
            ordered.add(new ObservedDevice(devices.get(index), scanObservations.get(index)));
        }
        Collections.sort(ordered, Comparator
                .comparingInt((ObservedDevice item) -> observationPriority(item.observation))
                .thenComparing(item -> item.observation.listEntry, String.CASE_INSENSITIVE_ORDER));
        devices.clear();
        scanObservations.clear();
        for (ObservedDevice item : ordered) {
            devices.add(item.device);
            scanObservations.add(item.observation);
        }
    }

    private int observationPriority(ScanObservation observation) {
        if (observation.remembered) {
            return 0;
        }
        if (observation.nameMatch) {
            return 1;
        }
        if (observation.bonded) {
            return 2;
        }
        return 3;
    }

    @SuppressLint("MissingPermission")
    private void refreshSelectionState() {
        if (selectedDevice != null) {
            String selectedAddress = selectedDevice.getAddress();
            for (int index = 0; index < devices.size(); index++) {
                BluetoothDevice device = devices.get(index);
                if (selectedAddress.equals(device.getAddress())) {
                    setSelectedScanDevice(device, scanObservations.get(index));
                    return;
                }
            }
        }

        BluetoothDevice preferredDevice = null;
        int matchedCount = 0;
        for (int index = 0; index < scanObservations.size(); index++) {
            ScanObservation observation = scanObservations.get(index);
            if (!observation.nameMatch) {
                continue;
            }
            matchedCount++;
            preferredDevice = devices.get(index);
        }
        if (matchedCount == 1 && preferredDevice != null) {
            int preferredIndex = devices.indexOf(preferredDevice);
            setSelectedScanDevice(
                    preferredDevice,
                    preferredIndex >= 0 ? scanObservations.get(preferredIndex) : null
            );
            return;
        }
        if (scanObservations.size() == 1) {
            setSelectedScanDevice(devices.get(0), scanObservations.get(0));
            return;
        }

        selectedDevice = null;
        selectedView.setText("Selected: none");
        connectButton.setEnabled(false);
    }

    @SuppressLint("MissingPermission")
    private void setSelectedScanDevice(BluetoothDevice device, ScanObservation observation) {
        selectedDevice = device;
        if (selectedDevice == null) {
            selectedView.setText("Selected: none");
            connectButton.setEnabled(false);
            return;
        }
        selectedView.setText("Selected: " + displayName(selectedDevice));
        connectButton.setEnabled(true);
        if (observation != null && !isProvisioningSessionConnected()) {
            resultView.setText("Selected candidate:\n" + observation.detailText);
        }
    }

    private boolean isProvisioningSessionConnected() {
        return (classicTransport != null && classicTransport.isConnected()) || gatt != null;
    }

    private boolean shouldIncludeScanResult(ScanObservation observation) {
        if (activeScanMode == ScanMode.GENERIC_TEST) {
            return true;
        }
        return observation.uuidMatch || observation.nameMatch;
    }

    @SuppressLint("MissingPermission")
    private ScanObservation describeClassicObservation(BluetoothDevice device, String discoveredName) {
        if (device == null) {
            return null;
        }

        String address = device.getAddress();
        boolean bonded = hasConnectPermission() && device.getBondState() == BluetoothDevice.BOND_BONDED;
        String deviceName = hasConnectPermission() ? sanitize(device.getName()) : "";
        String resolvedName = resolveClassicName(device, discoveredName);
        boolean nameMatch = startsWithLumelo(resolvedName);
        String preferredName = !resolvedName.isEmpty() ? resolvedName : "Classic Bluetooth device";

        StringBuilder listEntry = new StringBuilder();
        if (bonded) {
            listEntry.append("[PAIRED] ");
        }
        if (nameMatch) {
            listEntry.append("[NAME] ");
        } else {
            listEntry.append("[CLASSIC] ");
        }
        listEntry.append(preferredName).append(" (").append(address).append(")");

        StringBuilder detail = new StringBuilder();
        appendReportLine(detail, "Address", address);
        appendReportLine(detail, "Discovered Name", discoveredName);
        appendReportLine(detail, "Device Name", deviceName);
        appendReportLine(detail, "Resolved Name", resolvedName);
        appendReportLine(detail, "Bond State", bonded ? "bonded" : "not bonded");
        appendReportLine(detail, "Classic Match", nameMatch ? "yes" : "no");
        appendReportLine(detail, "Source", "classic_scan");
        appendReportLine(detail, "Transport", "classic_bluetooth");

        String logLine = "Classic scan result " + address
                + " bonded=" + (bonded ? "yes" : "no")
                + " nameMatch=" + (nameMatch ? "yes" : "no")
                + (resolvedName.isEmpty() ? "" : " name=" + resolvedName);

        return new ScanObservation(
                address,
                false,
                nameMatch,
                bonded,
                false,
                listEntry.toString(),
                detail.toString(),
                logLine
        );
    }

    @SuppressLint("MissingPermission")
    private ScanObservation describeRememberedClassicObservation(
            BluetoothDevice device,
            String rememberedName
    ) {
        if (device == null) {
            return null;
        }

        String address = device.getAddress();
        boolean bonded = hasConnectPermission() && device.getBondState() == BluetoothDevice.BOND_BONDED;
        String deviceName = hasConnectPermission() ? sanitize(device.getName()) : "";
        String preferredName = !rememberedName.isEmpty()
                ? rememberedName
                : (!deviceName.isEmpty() ? deviceName : "Lumelo T4");

        StringBuilder listEntry = new StringBuilder();
        if (bonded) {
            listEntry.append("[PAIRED] ");
        }
        listEntry.append("[LAST] [NAME] ")
                .append(preferredName)
                .append(" (")
                .append(address)
                .append(")");

        StringBuilder detail = new StringBuilder();
        appendReportLine(detail, "Address", address);
        appendReportLine(detail, "Remembered Name", preferredName);
        appendReportLine(detail, "Device Name", deviceName);
        appendReportLine(detail, "Bond State", bonded ? "bonded" : "not bonded");
        appendReportLine(detail, "Classic Match", "yes");
        appendReportLine(detail, "Source", "remembered_successful_device");
        appendReportLine(detail, "Transport", "classic_bluetooth");

        String logLine = "Remembered Lumelo device "
                + address
                + (preferredName.isEmpty() ? "" : " name=" + preferredName);

        return new ScanObservation(
                address,
                false,
                true,
                bonded,
                true,
                listEntry.toString(),
                detail.toString(),
                logLine
        );
    }

    @SuppressLint("MissingPermission")
    private ScanObservation describeScanObservation(ScanResult result) {
        BluetoothDevice device = result.getDevice();
        if (device == null) {
            return null;
        }

        ScanRecord scanRecord = result.getScanRecord();
        String address = device.getAddress();
        String localName = scanRecord == null ? "" : sanitize(scanRecord.getDeviceName());
        String deviceName = hasConnectPermission() ? sanitize(device.getName()) : "";
        String serviceUuids = scanRecord == null ? "" : formatServiceUuids(scanRecord);
        String manufacturerData = scanRecord == null ? "" : formatManufacturerData(scanRecord);
        boolean uuidMatch = scanRecord != null && hasLumeloServiceUuid(scanRecord);
        boolean nameMatch = startsWithLumelo(localName) || startsWithLumelo(deviceName);
        String preferredName = !localName.isEmpty()
                ? localName
                : (!deviceName.isEmpty()
                ? deviceName
                : (activeScanMode == ScanMode.GENERIC_TEST ? "BLE device" : "Lumelo candidate"));

        StringBuilder listEntry = new StringBuilder();
        if (uuidMatch) {
            listEntry.append("[UUID] ");
        }
        if (nameMatch) {
            listEntry.append("[NAME] ");
        }
        listEntry.append(preferredName)
                .append(" (")
                .append(address)
                .append(") RSSI ")
                .append(result.getRssi());
        if (activeScanMode == ScanMode.GENERIC_TEST) {
            appendIndentedLine(listEntry, "Local", localName);
            appendIndentedLine(listEntry, "Device", deviceName);
            appendIndentedLine(listEntry, "UUIDs", serviceUuids);
            appendIndentedLine(listEntry, "Mfg", shorten(manufacturerData, 72));
        }

        StringBuilder detail = new StringBuilder();
        appendReportLine(detail, "Address", address);
        appendReportLine(detail, "RSSI", String.valueOf(result.getRssi()));
        appendReportLine(detail, "Local Name", localName);
        appendReportLine(detail, "Device Name", deviceName);
        appendReportLine(detail, "UUID Match", uuidMatch ? "yes" : "no");
        appendReportLine(detail, "Name Match", nameMatch ? "yes" : "no");
        appendReportLine(detail, "Service UUIDs", serviceUuids);
        appendReportLine(detail, "Manufacturer Data", manufacturerData);

        String logLine = "Scan result " + address
                + " RSSI " + result.getRssi()
                + " uuidMatch=" + (uuidMatch ? "yes" : "no")
                + " nameMatch=" + (nameMatch ? "yes" : "no")
                + (localName.isEmpty() ? "" : " local=" + localName)
                + (serviceUuids.isEmpty() ? "" : " uuids=" + shorten(serviceUuids, 48));

        return new ScanObservation(
                address,
~~~

