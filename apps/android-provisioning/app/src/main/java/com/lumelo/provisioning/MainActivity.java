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
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.net.ConnectivityManager;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.Editable;
import android.text.InputType;
import android.text.TextWatcher;
import android.util.SparseArray;
import android.view.ViewGroup;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ListView;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import java.io.IOException;
import java.io.InputStream;
import java.io.ByteArrayOutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.Date;
import java.util.List;
import java.util.Locale;
import java.util.UUID;
import java.util.concurrent.ExecutorCompletionService;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;

public class MainActivity extends Activity {
    private static final String PREFS_NAME = "lumelo_setup";
    private static final String PREF_LAST_WEB_URL = "last_web_url";
    private static final String PREF_LAST_T4_SSID = "last_t4_ssid";
    private static final String PREF_LAST_CLASSIC_ADDRESS = "last_classic_address";
    private static final String PREF_LAST_CLASSIC_NAME = "last_classic_name";
    private static final String PREF_LAST_LUMELO_LOCAL_SUPPORTED = "last_lumelo_local_supported";
    private static final String LUMELO_LOCAL_BASE_URL = "http://lumelo.local/";
    private static final String LUMELO_LOCAL_HEALTHZ_URL = "http://lumelo.local/healthz";
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
    private static final int RSSI_UNKNOWN = Integer.MIN_VALUE;
    private static final int WEBUI_PROBE_TIMEOUT_MS = 2_000;
    private static final int WEBUI_SUBNET_SCAN_TIMEOUT_MS = 800;
    private static final int WEBUI_SUBNET_SCAN_WORKERS = 12;

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
    private TextView classicSessionView;
    private TextView scanSummaryView;
    private TextView selectedView;
    private TextView deviceInfoView;
    private TextView resultView;
    private TextView credentialFormView;
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
    private Button copySummaryButton;
    private EditText ssidInput;
    private EditText passwordInput;
    private String webUrl;
    private boolean mainInterfaceOpenedForSession;
    private String passwordEditedForSsid = "";
    private volatile String lastStatusText = "";
    private volatile String lastWebUiProbeSummary = "";
    private volatile String lastSubnetScanSummary = "";
    private long classicFailureProbeSerial;
    private long webEntryProbeSerial;
    private long statusPollingDeadlineMs;
    private boolean statusPollingActive;
    private boolean webEntryResolvedForSession;
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
                    short intentRssi = intent.getShortExtra(BluetoothDevice.EXTRA_RSSI, Short.MIN_VALUE);
                    handleClassicScanResult(
                            device,
                            sanitize(intent.getStringExtra(BluetoothDevice.EXTRA_NAME)),
                            intentRssi == Short.MIN_VALUE ? RSSI_UNKNOWN : intentRssi
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

        classicSessionView = label("Classic session: idle");
        root.addView(classicSessionView, matchWrap());

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
        connectButton.setOnClickListener(view -> {
            if (classicTransport != null && classicTransport.isConnectInProgress()) {
                disconnectFromDevice();
            } else {
                connectSelectedDevice();
            }
        });
        root.addView(connectButton, matchWrap());

        deviceInfoView = label("Device info: not connected");
        root.addView(deviceInfoView, matchWrap());

        ssidInput = input("Wi-Fi SSID", false);
        root.addView(ssidInput, matchWrap());

        passwordInput = input("Wi-Fi password", true);
        root.addView(passwordInput, matchWrap());

        credentialFormView = label("Credential form: waiting for SSID and password.");
        root.addView(credentialFormView, matchWrap());

        ssidInput.addTextChangedListener(simpleTextWatcher(this::refreshCredentialFormSummary));
        passwordInput.addTextChangedListener(new TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence s, int start, int count, int after) {
            }

            @Override
            public void onTextChanged(CharSequence s, int start, int before, int count) {
            }

            @Override
            public void afterTextChanged(Editable editable) {
                passwordEditedForSsid = sanitize(ssidInput.getText().toString());
                refreshCredentialFormSummary();
            }
        });

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

        copySummaryButton = button("Copy Diagnostic Summary");
        copySummaryButton.setOnClickListener(view -> copyDiagnosticSummary());
        root.addView(copySummaryButton, matchWrap());

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
                cancelClassicFailureProbe();
                clearConnectivityDiagnostics();
                rememberClassicDevice(device);
                runOnUiThread(() -> {
                    sendButton.setEnabled(true);
                    readStatusButton.setEnabled(true);
                    disconnectButton.setEnabled(true);
                    refreshConnectButtonState();
                });
                setStatus("Classic Bluetooth connected. Reading T4 info...");
                logEvent("Classic Bluetooth connected to " + displayName(device));
            }

            @Override
            public void onClassicReconnecting(String message) {
                cancelClassicFailureProbe();
                stopStatusPolling();
                runOnUiThread(() -> {
                    sendButton.setEnabled(false);
                    readStatusButton.setEnabled(false);
                    disconnectButton.setEnabled(true);
                    refreshConnectButtonState();
                });
                setStatus(withRememberedWebUiHint(message));
                logEvent(message);
            }

            @Override
            public void onClassicDisconnected(String message) {
                cancelClassicFailureProbe();
                stopStatusPolling();
                resetProvisioningSession();
                setStatus(withRememberedWebUiHint(message));
                logEvent(message);
            }

            @Override
            public void onClassicDeviceInfo(String payload) {
                runOnUiThread(() -> deviceInfoView.setText(formatDeviceInfoText(payload)));
                rememberReportedWebUiUrl(deriveWebUiUrl(
                        "",
                        extractJsonString(payload, "ip"),
                        extractJsonString(payload, "wifi_ip"),
                        extractJsonString(payload, "wired_ip"),
                        extractJsonInt(payload, "web_port")
                ));
                logEvent("Classic device info payload: " + payload);
            }

            @Override
            public void onClassicStatus(String payload) {
                showResult(payload);
            }

            @Override
            public void onClassicError(String message) {
                setStatus(withClassicConnectRecoveryHint(message));
                if (isClassicConnectFailureMessage(message)) {
                    stopStatusPolling();
                    refreshClassicConnectFailureUi();
                }
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
                            Manifest.permission.ACCESS_COARSE_LOCATION,
                            Manifest.permission.ACCESS_FINE_LOCATION,
                    },
                    REQUEST_BLE_PERMISSIONS
            );
        } else {
            requestPermissions(
                    new String[]{
                            Manifest.permission.ACCESS_COARSE_LOCATION,
                            Manifest.permission.ACCESS_FINE_LOCATION,
                    },
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
        refreshConnectButtonState();
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
            int rememberedDevices = countRememberedCandidates();
            if (devices.isEmpty()) {
                setStatus("Scan finished. No Lumelo device found.");
                logEvent("Classic Bluetooth scan finished: no Lumelo device found");
            } else if (matchedDevices == 0) {
                if (rememberedDevices == 1 && selectedDevice != null) {
                    setStatus("Scan finished. No named Lumelo device found. Selected the last successful T4 candidate.");
                    logEvent("Classic Bluetooth scan finished: no named Lumelo match; selected 1 remembered classic candidate");
                } else if (rememberedDevices > 0) {
                    setStatus("Scan finished. No named Lumelo device found. Showing last successful T4 candidate(s) first.");
                    logEvent("Classic Bluetooth scan finished: no named Lumelo match; showing "
                            + rememberedDevices + " remembered classic candidate(s) first");
                } else {
                    setStatus("Scan finished. No named Lumelo device found. Showing classic candidates.");
                    logEvent("Classic Bluetooth scan finished: no named Lumelo match; showing "
                            + devices.size() + " classic candidate(s)");
                }
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
            ScanObservation observation = describeClassicObservation(bondedDevice, "", RSSI_UNKNOWN);
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
        } catch (IllegalArgumentException exception) {
            logEvent("Ignoring invalid remembered classic address: " + rememberedAddress);
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
        handleClassicScanResult(device, discoveredName, RSSI_UNKNOWN);
    }

    private void handleClassicScanResult(BluetoothDevice device, String discoveredName, int rssi) {
        int effectiveRssi = rssi;
        if (effectiveRssi == RSSI_UNKNOWN) {
            effectiveRssi = knownRssiFor(device == null ? "" : device.getAddress());
        }
        ScanObservation observation = describeClassicObservation(device, discoveredName, effectiveRssi);
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

        BluetoothDevice rememberedDevice = null;
        int rememberedCount = 0;
        for (int index = 0; index < scanObservations.size(); index++) {
            ScanObservation observation = scanObservations.get(index);
            if (!observation.remembered) {
                continue;
            }
            rememberedCount++;
            rememberedDevice = devices.get(index);
        }
        if (matchedCount == 0 && rememberedCount == 1 && rememberedDevice != null) {
            int rememberedIndex = devices.indexOf(rememberedDevice);
            setSelectedScanDevice(
                    rememberedDevice,
                    rememberedIndex >= 0 ? scanObservations.get(rememberedIndex) : null
            );
            return;
        }

        selectedDevice = null;
        selectedView.setText("Selected: none");
        refreshConnectButtonState();
    }

    @SuppressLint("MissingPermission")
    private void setSelectedScanDevice(BluetoothDevice device, ScanObservation observation) {
        selectedDevice = device;
        if (selectedDevice == null) {
            selectedView.setText("Selected: none");
            refreshConnectButtonState();
            return;
        }
        if (observation != null && observation.remembered && !observation.nameMatch) {
            selectedView.setText("Selected: last successful T4 candidate " + displayName(selectedDevice));
        } else {
            selectedView.setText("Selected: " + displayName(selectedDevice));
        }
        refreshConnectButtonState();
        if (observation != null && !isProvisioningSessionConnected()) {
            resultView.setText("Selected candidate:\n" + observation.detailText);
        }
    }

    private boolean isProvisioningSessionConnected() {
        return (classicTransport != null && classicTransport.isConnected()) || gatt != null;
    }

    private void refreshConnectButtonState() {
        if (connectButton == null) {
            return;
        }
        boolean connectPending = classicTransport != null && classicTransport.isConnectInProgress();
        if (connectPending) {
            connectButton.setText("Cancel Connect");
            connectButton.setEnabled(true);
            return;
        }
        connectButton.setText("Connect");
        connectButton.setEnabled(selectedDevice != null && !isProvisioningSessionConnected());
    }

    private boolean shouldIncludeScanResult(ScanObservation observation) {
        if (activeScanMode == ScanMode.GENERIC_TEST) {
            return true;
        }
        return observation.uuidMatch || observation.nameMatch;
    }

    @SuppressLint("MissingPermission")
    private ScanObservation describeClassicObservation(BluetoothDevice device, String discoveredName, int rssi) {
        if (device == null) {
            return null;
        }

        String address = device.getAddress();
        boolean bonded = hasConnectPermission() && device.getBondState() == BluetoothDevice.BOND_BONDED;
        boolean remembered = isRememberedClassicAddress(address);
        String deviceName = hasConnectPermission() ? sanitize(device.getName()) : "";
        String resolvedName = resolveClassicName(device, discoveredName);
        boolean nameMatch = startsWithLumelo(resolvedName);
        String preferredName = !resolvedName.isEmpty() ? resolvedName : "Classic Bluetooth candidate";
        String selectionHint = classicSelectionHint(remembered, nameMatch, bonded);
        String rssiLabel = formatRssi(rssi);

        StringBuilder listEntry = new StringBuilder();
        if (remembered) {
            listEntry.append("[LAST] ");
        }
        if (bonded) {
            listEntry.append("[PAIRED] ");
        }
        if (nameMatch) {
            listEntry.append("[NAME] ");
        } else {
            listEntry.append("[CLASSIC] ");
        }
        listEntry.append(preferredName).append(" (").append(address).append(")");
        if (!rssiLabel.isEmpty()) {
            listEntry.append(" RSSI ").append(rssiLabel);
        }

        StringBuilder detail = new StringBuilder();
        appendReportLine(detail, "Address", address);
        appendReportLine(detail, "MAC Suffix", address.length() >= 5 ? address.substring(address.length() - 5) : address);
        appendReportLine(detail, "Discovered Name", discoveredName);
        appendReportLine(detail, "Device Name", deviceName);
        appendReportLine(detail, "Resolved Name", resolvedName);
        appendReportLine(detail, "RSSI", rssiLabel);
        appendReportLine(detail, "Bond State", bonded ? "bonded" : "not bonded");
        appendReportLine(detail, "Last Successful T4", remembered ? "yes" : "no");
        appendReportLine(detail, "Classic Match", nameMatch ? "yes" : "no");
        appendReportLine(detail, "Selection Hint", selectionHint);
        appendReportLine(detail, "Source", "classic_scan");
        appendReportLine(detail, "Transport", "classic_bluetooth");

        String logLine = "Classic scan result " + address
                + " bonded=" + (bonded ? "yes" : "no")
                + " remembered=" + (remembered ? "yes" : "no")
                + " nameMatch=" + (nameMatch ? "yes" : "no")
                + (rssiLabel.isEmpty() ? "" : " rssi=" + rssiLabel)
                + (resolvedName.isEmpty() ? "" : " name=" + resolvedName);

        return new ScanObservation(
                address,
                false,
                nameMatch,
                bonded,
                remembered,
                rssi,
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
        appendReportLine(detail, "MAC Suffix", address.length() >= 5 ? address.substring(address.length() - 5) : address);
        appendReportLine(detail, "Remembered Name", preferredName);
        appendReportLine(detail, "Device Name", deviceName);
        appendReportLine(detail, "Bond State", bonded ? "bonded" : "not bonded");
        appendReportLine(detail, "Last Successful T4", "yes");
        appendReportLine(detail, "Classic Match", "yes");
        appendReportLine(detail, "Selection Hint", "This phone connected to this T4 successfully before.");
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
                RSSI_UNKNOWN,
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
                uuidMatch,
                nameMatch,
                false,
                false,
                result.getRssi(),
                listEntry.toString(),
                detail.toString(),
                logLine
        );
    }

    private void refreshScanSummary() {
        int uuidMatched = 0;
        int nameMatched = 0;
        int paired = 0;
        int remembered = 0;
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
            if (observation.remembered) {
                remembered++;
            }
        }
        String modeLabel = activeScanMode == ScanMode.LUMELO
                ? "Classic Bluetooth scan"
                : "Raw BLE scan";
        String selectedAddress = selectedDevice == null ? "none" : selectedDevice.getAddress();
        scanSummaryView.setText(String.format(
                Locale.US,
                "Scan summary: %s | devices=%d | uuidMatch=%d | nameMatch=%d | paired=%d | remembered=%d | selected=%s",
                modeLabel,
                scanObservations.size(),
                uuidMatched,
                nameMatched,
                paired,
                remembered,
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

    private int countRememberedCandidates() {
        int count = 0;
        for (ScanObservation observation : scanObservations) {
            if (observation.remembered) {
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

    private boolean isRememberedClassicAddress(String address) {
        if (address == null || address.isEmpty()) {
            return false;
        }
        String rememberedAddress = prefs().getString(PREF_LAST_CLASSIC_ADDRESS, "");
        return rememberedAddress != null && rememberedAddress.equalsIgnoreCase(address);
    }

    private int knownRssiFor(String address) {
        if (address == null || address.isEmpty()) {
            return RSSI_UNKNOWN;
        }
        for (ScanObservation observation : scanObservations) {
            if (address.equalsIgnoreCase(observation.address) && observation.rssi != RSSI_UNKNOWN) {
                return observation.rssi;
            }
        }
        return RSSI_UNKNOWN;
    }

    private String classicSelectionHint(boolean remembered, boolean nameMatch, boolean bonded) {
        if (remembered) {
            return "This phone connected to this T4 successfully before.";
        }
        if (nameMatch) {
            return "Device name looks like Lumelo / NanoPC-T4.";
        }
        if (bonded) {
            return "Paired classic candidate. If unsure, compare the MAC suffix.";
        }
        return "Anonymous classic candidate. Compare the MAC suffix or retry scan.";
    }

    private String formatRssi(int rssi) {
        if (rssi == RSSI_UNKNOWN) {
            return "";
        }
        return String.valueOf(rssi);
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
        cancelClassicFailureProbe();
        stopStatusPolling();
        clearConnectivityDiagnostics();
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
                runOnUiThread(() -> {
                    disconnectButton.setEnabled(true);
                    refreshConnectButtonState();
                });
            }
            return;
        }
        gatt = selectedDevice.connectGatt(this, false, gattCallback, BluetoothDevice.TRANSPORT_LE);
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
                runOnUiThread(() -> deviceInfoView.setText(formatDeviceInfoText(text)));
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
                runOnUiThread(() -> deviceInfoView.setText(formatDeviceInfoText(text)));
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
                setStatus(withRememberedWebUiHint("Connect to Lumelo T4 before reading status."));
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
        String passwordKind = describePasswordType(password);
        if (ssid.isEmpty()) {
            setStatus("SSID is required.");
            logEvent("Cannot send credentials: empty SSID");
            return;
        }
        if (!isValidWpaPassword(password)) {
            setStatus("WPA password must be 8..63 characters, or 64 hexadecimal digits.");
            logEvent("Cannot send credentials: password length invalid");
            return;
        }
        if (passwordEnteredForDifferentSsid(ssid, password)) {
            setStatus("SSID changed after password entry. Re-check the Wi-Fi password before sending.");
            logEvent("Cannot send credentials: password was entered for a different SSID");
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
            setStatus("Sent " + passwordKind + " credentials for SSID " + ssid + ". Waiting for T4 status...");
            logEvent("Queued " + passwordKind + " credentials for SSID "
                    + ssid
                    + " over classic Bluetooth (password length="
                    + password.length()
                    + ")");
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
        setStatus("Sent " + passwordKind + " credentials for SSID " + ssid + ". Waiting for T4 status...");
        logEvent("Queued " + passwordKind + " credentials for SSID "
                + ssid
                + " over BLE (password length="
                + password.length()
                + ")");
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
            return checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) == PackageManager.PERMISSION_GRANTED
                    && hasLocationPermission();
        }
        return hasLocationPermission();
    }

    private boolean hasConnectPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            return checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED;
        }
        return true;
    }

    private boolean hasLocationPermission() {
        return checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED
                || checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED;
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
        String wifiIp = extractJsonString(text, "wifi_ip");
        String wiredIp = extractJsonString(text, "wired_ip");
        String wifiInterface = extractJsonString(text, "wifi_interface");
        String errorCode = extractJsonString(text, "error_code");
        String applyOutput = extractJsonString(text, "apply_output");
        String diagnosticHint = extractJsonString(text, "diagnostic_hint");
        String wpaUnit = extractJsonString(text, "wpa_unit");
        int ipWaitSeconds = extractJsonInt(text, "ip_wait_seconds");
        runOnUiThread(() -> {
            String candidateWebUrl = deriveWebUiUrl(
                    extractJsonString(text, "web_url"),
                    ip,
                    wifiIp,
                    wiredIp,
                    extractJsonInt(text, "web_port")
            );
            if (ssid != null && !ssid.isEmpty()) {
                persistT4Ssid(ssid);
            }
            rememberReportedWebUiUrl(candidateWebUrl);
            resultView.setText(formatResultText(
                    state,
                    message,
                    ssid,
                    ip,
                    wifiIp,
                    wiredIp,
                    wifiInterface,
                    errorCode,
                    applyOutput,
                    diagnosticHint,
                    wpaUnit,
                    ipWaitSeconds,
                    text
            ));
            setWebButtonsEnabled(webUrl != null && !webUrl.isEmpty());
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
        lastStatusText = text == null ? "" : text;
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
        } else if (classicTransport != null && classicTransport.isConnectInProgress()) {
            sessionState = "classic_connecting";
        } else if (statusCharacteristic != null) {
            sessionState = "gatt_ready";
        } else if (gatt != null) {
            sessionState = "gatt_connected";
        } else {
            sessionState = "idle";
        }

        String phoneWifi = currentConnectedWifiSsid();
        if (phoneWifi.isEmpty()) {
            phoneWifi = "(unknown)";
        }
        String lastT4Wifi = rememberedT4Ssid();
        if (lastT4Wifi.isEmpty()) {
            lastT4Wifi = "(unknown)";
        }

        return String.format(
                Locale.US,
                "Environment: %s, scanPerm=%s, connectPerm=%s, session=%s, mtu=%d, sdk=%d, phoneWiFi=%s, lastT4WiFi=%s",
                adapterState,
                hasScanPermission() ? "granted" : "missing",
                hasConnectPermission() ? "granted" : "missing",
                sessionState,
                negotiatedMtu,
                Build.VERSION.SDK_INT,
                phoneWifi,
                lastT4Wifi
        );
    }

    private String buildClassicSessionSummary() {
        if (classicTransport == null) {
            return "Classic session: unavailable";
        }
        return "Classic session: " + classicTransport.debugSummary();
    }

    private void refreshEnvironmentStatus() {
        String summary = buildEnvironmentSummary();
        String classicSummary = buildClassicSessionSummary();
        runOnUiThread(() -> {
            environmentView.setText(summary);
            if (classicSessionView != null) {
                classicSessionView.setText(classicSummary);
            }
        });
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

    private void copyDiagnosticSummary() {
        ClipboardManager clipboardManager = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (clipboardManager == null) {
            setStatus("Clipboard service unavailable on this phone.");
            logEvent("Clipboard service unavailable");
            return;
        }
        String summary = buildQuickDiagnosticSummary();
        clipboardManager.setPrimaryClip(ClipData.newPlainText("Lumelo diagnostic summary", summary));
        runOnUiThread(() -> Toast.makeText(
                MainActivity.this,
                "Copied diagnostic summary",
                Toast.LENGTH_SHORT
        ).show());
        logEvent("Copied current diagnostic summary to clipboard");
    }

    private String buildDiagnosticReport() {
        StringBuilder builder = new StringBuilder();
        builder.append(buildQuickDiagnosticSummary()).append("\n\n");
        builder.append("Lumelo Setup Diagnostic\n");
        appendReportLine(builder, "Build", buildBuildInfoText());
        appendReportLine(builder, "Status", statusView.getText().toString());
        appendReportLine(builder, "Environment", buildEnvironmentSummary());
        appendReportLine(builder, "Classic Session", buildClassicSessionSummary());
        appendReportLine(builder, "Credential Form", buildCredentialFormSummary());
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

    private String buildQuickDiagnosticSummary() {
        StringBuilder builder = new StringBuilder();
        builder.append("Lumelo Setup Quick Summary\n");
        appendReportLine(builder, "Build", buildBuildInfoText());
        appendReportLine(builder, "Status", sanitize(lastStatusText));
        appendReportLine(builder, "Selected", sanitize(selectedView == null ? "" : selectedView.getText().toString()));
        appendReportLine(builder, "Phone Wi-Fi", currentConnectedWifiSsid());
        Ipv4Network.AddressInfo phoneInfo = Ipv4Network.currentAddressInfo(
                getSystemService(ConnectivityManager.class)
        );
        if (phoneInfo != null) {
            appendReportLine(builder, "Phone IPv4", phoneInfo.address + "/" + phoneInfo.prefixLength);
        }
        appendReportLine(builder, "Last known T4 Wi-Fi", rememberedT4Ssid());
        appendReportLine(builder, "Last known WebUI", currentKnownWebUiUrl());
        appendReportLine(builder, "lumelo.local supported", rememberedLumeloLocalSupportLabel());
        appendReportLine(builder, "WebUI probe", sanitize(lastWebUiProbeSummary));
        appendReportLine(builder, "Subnet scan", sanitize(lastSubnetScanSummary));
        appendReportLine(builder, "Classic Session", buildClassicSessionSummary());
        return builder.toString();
    }

    private String rememberedLumeloLocalSupportLabel() {
        if (!prefs().contains(PREF_LAST_LUMELO_LOCAL_SUPPORTED)) {
            return "unknown";
        }
        return prefs().getBoolean(PREF_LAST_LUMELO_LOCAL_SUPPORTED, false) ? "yes" : "no";
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

    private TextWatcher simpleTextWatcher(Runnable afterChanged) {
        return new TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence s, int start, int count, int after) {
            }

            @Override
            public void onTextChanged(CharSequence s, int start, int before, int count) {
            }

            @Override
            public void afterTextChanged(Editable editable) {
                afterChanged.run();
            }
        };
    }

    private void refreshCredentialFormSummary() {
        if (credentialFormView == null) {
            return;
        }
        runOnUiThread(() -> credentialFormView.setText(buildCredentialFormSummary()));
    }

    private String buildCredentialFormSummary() {
        String ssid = sanitize(ssidInput == null ? "" : ssidInput.getText().toString());
        String password = passwordInput == null ? "" : passwordInput.getText().toString();
        String phoneWifi = currentConnectedWifiSsid();
        String lastT4Wifi = rememberedT4Ssid();

        StringBuilder builder = new StringBuilder("Credential form:");
        appendInlineSummary(builder, "Target SSID", ssid.isEmpty() ? "(empty)" : ssid);
        appendInlineSummary(builder, "Password length", String.valueOf(password.length()));
        appendInlineSummary(builder, "Password type", describePasswordType(password));
        appendInlineSummary(builder, "Phone Wi-Fi", phoneWifi.isEmpty() ? "(unknown)" : phoneWifi);
        if (!lastT4Wifi.isEmpty()) {
            appendInlineSummary(builder, "Last T4 Wi-Fi", lastT4Wifi);
        }
        if (passwordEnteredForDifferentSsid(ssid, password)) {
            appendInlineSummary(builder, "Warning", "SSID changed after password entry; re-check before sending");
        } else if (!ssid.isEmpty() && !phoneWifi.isEmpty() && !ssid.equals(phoneWifi)) {
            appendInlineSummary(builder, "Note", "phone is not currently on the target SSID");
        }
        return builder.toString();
    }

    private void appendInlineSummary(StringBuilder builder, String label, String value) {
        if (value == null || value.isEmpty()) {
            return;
        }
        builder.append("\n").append(label).append(": ").append(value);
    }

    private boolean passwordEnteredForDifferentSsid(String currentSsid, String password) {
        if (password == null || password.isEmpty()) {
            return false;
        }
        String normalizedCurrent = sanitize(currentSsid);
        String editedFor = sanitize(passwordEditedForSsid);
        if (normalizedCurrent.isEmpty() || editedFor.isEmpty()) {
            return false;
        }
        return !normalizedCurrent.equals(editedFor);
    }

    private String describePasswordType(String password) {
        if (password == null || password.isEmpty()) {
            return "(empty)";
        }
        if (password.length() == 64 && isHex(password)) {
            return "64-char hex PSK";
        }
        if (password.length() < 8) {
            return "too short";
        }
        if (password.length() > 63) {
            return "too long";
        }
        return "WPA passphrase";
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

    private String formatDeviceInfoText(String payload) {
        String name = extractJsonString(payload, "name");
        String hostname = extractJsonString(payload, "hostname");
        String ip = extractJsonString(payload, "ip");
        String wifiIp = extractJsonString(payload, "wifi_ip");
        String wiredIp = extractJsonString(payload, "wired_ip");
        String wifiInterface = extractJsonString(payload, "wifi_interface");
        String transport = extractJsonString(payload, "transport");
        String statusPath = extractJsonString(payload, "status_path");
        int webPort = extractJsonInt(payload, "web_port");

        StringBuilder builder = new StringBuilder("Device info:");
        appendResultLine(builder, "Name", name);
        appendResultLine(builder, "Hostname", hostname);
        appendResultLine(builder, "IP", ip);
        appendResultLine(builder, "Wi-Fi IP", wifiIp);
        appendResultLine(builder, "Wired IP", wiredIp);
        appendResultLine(builder, "Wi-Fi interface", wifiInterface);
        appendResultLine(builder, "Transport", transport);
        appendResultLine(builder, "Status path", statusPath);
        if (webPort > 0) {
            appendResultLine(builder, "Web port", String.valueOf(webPort));
        }
        appendResultLine(builder, "Raw", payload);
        return builder.toString();
    }

    private String deriveWebUiUrl(
            String reportedWebUrl,
            String ip,
            String wifiIp,
            String wiredIp,
            int webPort
    ) {
        String normalizedUrl = sanitize(reportedWebUrl);
        if (!normalizedUrl.isEmpty()) {
            return normalizedUrl;
        }

        String reachableIp = sanitize(ip);
        if (reachableIp.isEmpty()) {
            reachableIp = sanitize(wifiIp);
        }
        if (reachableIp.isEmpty()) {
            reachableIp = sanitize(wiredIp);
        }
        if (reachableIp.isEmpty()) {
            return "";
        }

        int resolvedPort = webPort > 0 ? webPort : 80;
        if (resolvedPort == 80) {
            return "http://" + reachableIp + "/";
        }
        return "http://" + reachableIp + ":" + resolvedPort + "/";
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
            if (classicTransport == null
                    || (!classicTransport.isConnected() && !classicTransport.isConnectInProgress())) {
                setStatus("No active classic Bluetooth connection to disconnect.");
                return;
            }
            stopStatusPolling();
            classicTransport.disconnect();
            resetProvisioningSession();
            setStatus(withRememberedWebUiHint("Disconnected from T4."));
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
                ? withRememberedWebUiHint("Disconnected from T4.")
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
        try {
            currentGatt.close();
        } catch (SecurityException exception) {
            logEvent("Cannot close GATT session: " + exception.getClass().getSimpleName());
        }
    }

    private void finishGattSession(BluetoothGatt bluetoothGatt) {
        if (gatt == bluetoothGatt) {
            gatt = null;
        }
        if (hasConnectPermission()) {
            try {
                bluetoothGatt.close();
            } catch (SecurityException exception) {
                logEvent("Cannot close finished GATT session: " + exception.getClass().getSimpleName());
            }
        } else {
            logEvent("Cannot close GATT session: missing Bluetooth connect permission");
        }
        resetProvisioningSession();
    }

    private void resetProvisioningSession() {
        cancelClassicFailureProbe();
        cancelWebEntryProbe();
        stopStatusPolling();
        deviceInfoCharacteristic = null;
        wifiCredentialsCharacteristic = null;
        applyCharacteristic = null;
        statusCharacteristic = null;
        webUrl = rememberedWebUiUrl();
        mainInterfaceOpenedForSession = false;
        webEntryResolvedForSession = false;
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
            resultView.setText(webUrl != null && !webUrl.isEmpty()
                    ? buildRememberedWebUiText(webUrl)
                    : "Result: waiting");
            refreshConnectButtonState();
        });
    }

    private boolean maybeOpenMainInterface(String state) {
        if (!"connected".equals(state) || mainInterfaceOpenedForSession || webUrl == null || webUrl.isEmpty()) {
            return false;
        }
        if (webEntryResolvedForSession) {
            return false;
        }
        webEntryResolvedForSession = true;
        boolean autoOpen = shouldAutoOpenWebUi();
        if (autoOpen) {
            mainInterfaceOpenedForSession = true;
        }
        resolvePreferredWebEntryAsync(webUrl, autoOpen);
        return autoOpen;
    }

    private synchronized long cancelWebEntryProbe() {
        webEntryProbeSerial += 1;
        return webEntryProbeSerial;
    }

    private void resolvePreferredWebEntryAsync(String fallbackUrl, boolean openAfterProbe) {
        String normalizedFallback = sanitize(fallbackUrl);
        if (normalizedFallback.isEmpty()) {
            return;
        }
        final long probeSerial = cancelWebEntryProbe();
        setStatus("T4 connected. Checking http://lumelo.local/ ...");
        logEvent("Probing default WebUI entry " + LUMELO_LOCAL_HEALTHZ_URL);
        new Thread(() -> {
            boolean localReachable = probeLumeloHealthzUrl(LUMELO_LOCAL_HEALTHZ_URL, WEBUI_PROBE_TIMEOUT_MS);
            synchronized (this) {
                if (probeSerial != webEntryProbeSerial) {
                    return;
                }
            }
            String selectedUrl = localReachable ? LUMELO_LOCAL_BASE_URL : normalizedFallback;
            String summary = localReachable
                    ? "lumelo.local reachable; using default entry"
                    : "lumelo.local unreachable; using reliable IP entry";
            lastWebUiProbeSummary = summary;
            prefs().edit().putBoolean(PREF_LAST_LUMELO_LOCAL_SUPPORTED, localReachable).apply();
            runOnUiThread(() -> {
                rememberReportedWebUiUrl(selectedUrl);
                String status = localReachable
                        ? "T4 connected. Default entry http://lumelo.local/ works on this phone."
                        : "T4 connected. http://lumelo.local/ is not reachable on this phone; using " + normalizedFallback;
                setStatus(status);
                if (openAfterProbe) {
                    logEvent("Opening Lumelo main interface via " + selectedUrl);
                    openInAppUrl(selectedUrl);
                }
            });
        }, "LumeloLocalProbe").start();
    }

    private void openInAppUrl(String url) {
        if (url == null || url.isEmpty()) {
            setStatus("No WebUI URL reported yet.");
            return;
        }
        persistWebUiUrl(url);
        Intent intent = new Intent(this, MainInterfaceActivity.class);
        intent.putExtra(MainInterfaceActivity.EXTRA_INITIAL_URL, url);
        String rememberedSsid = rememberedT4Ssid();
        if (!rememberedSsid.isEmpty()) {
            intent.putExtra(MainInterfaceActivity.EXTRA_EXPECTED_T4_SSID, rememberedSsid);
        }
        startActivity(intent);
    }

    private void setWebButtonsEnabled(boolean enabled) {
        openWebButton.setEnabled(enabled);
        openProvisioningButton.setEnabled(enabled);
        openLogsButton.setEnabled(enabled);
        openHealthzButton.setEnabled(enabled);
    }

    private void rememberReportedWebUiUrl(String candidateUrl) {
        String normalized = sanitize(candidateUrl);
        if (normalized.isEmpty()) {
            return;
        }
        webUrl = normalized;
        persistWebUiUrl(normalized);
        runOnUiThread(() -> setWebButtonsEnabled(true));
    }

    private void persistWebUiUrl(String url) {
        if (url == null || url.isEmpty()) {
            return;
        }
        prefs().edit().putString(PREF_LAST_WEB_URL, url).apply();
    }

    private void persistT4Ssid(String ssid) {
        String normalized = sanitize(ssid);
        if (normalized.isEmpty()) {
            return;
        }
        prefs().edit().putString(PREF_LAST_T4_SSID, normalized).apply();
    }

    private String rememberedT4Ssid() {
        return sanitize(prefs().getString(PREF_LAST_T4_SSID, ""));
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
                .apply();
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
        String rememberedUrl = rememberedWebUiUrl();
        if (rememberedUrl == null || rememberedUrl.isEmpty()) {
            return;
        }
        webUrl = rememberedUrl;
        runOnUiThread(() -> {
            setWebButtonsEnabled(true);
            if (!disconnectButton.isEnabled()) {
                resultView.setText(buildRememberedWebUiText(rememberedUrl));
            }
        });
    }

    private String rememberedWebUiUrl() {
        return sanitize(prefs().getString(PREF_LAST_WEB_URL, ""));
    }

    private String currentKnownWebUiUrl() {
        String current = sanitize(webUrl);
        if (!current.isEmpty()) {
            return current;
        }
        return rememberedWebUiUrl();
    }

    private String buildRememberedWebUiText(String rememberedUrl) {
        String rememberedSsid = rememberedT4Ssid();
        return "Result:\nLast known WebUI: " + rememberedUrl
                + (rememberedSsid.isEmpty()
                ? ""
                : "\nLast known T4 Wi-Fi: " + rememberedSsid)
                + "\nTip: if phone and T4 are on the same network, you can open it directly.";
    }

    private String withRememberedWebUiHint(String message) {
        String rememberedUrl = webUrl;
        if (rememberedUrl == null || rememberedUrl.isEmpty()) {
            rememberedUrl = rememberedWebUiUrl();
        }
        if (rememberedUrl == null || rememberedUrl.isEmpty()) {
            return message;
        }
        return message + " Last known WebUI: " + rememberedUrl;
    }

    private String withClassicConnectRecoveryHint(String message) {
        String status = withRememberedWebUiHint(message);
        if (message == null) {
            return status;
        }
        String normalized = message.toLowerCase(Locale.ROOT);
        if (!normalized.startsWith("classic bluetooth connect failed")) {
            return status;
        }
        if (status.contains("Try OPEN WEBUI")) {
            return status;
        }
        String rememberedUrl = webUrl;
        if (rememberedUrl == null || rememberedUrl.isEmpty()) {
            rememberedUrl = rememberedWebUiUrl();
        }
        if (rememberedUrl != null && !rememberedUrl.isEmpty()) {
            return status + " Try OPEN WEBUI if phone and T4 are still on the same network.";
        }
        return status + " Scan still sees Lumelo, so check whether the T4 classic provisioning service is up.";
    }

    private synchronized long cancelClassicFailureProbe() {
        classicFailureProbeSerial += 1;
        return classicFailureProbeSerial;
    }

    private boolean isClassicConnectFailureMessage(String message) {
        if (message == null) {
            return false;
        }
        String normalized = message.toLowerCase(Locale.ROOT);
        return normalized.startsWith("classic bluetooth connect failed")
                || normalized.startsWith("classic bluetooth stream setup failed");
    }

    private void refreshClassicConnectFailureUi() {
        String rememberedUrl = webUrl;
        if (rememberedUrl == null || rememberedUrl.isEmpty()) {
            rememberedUrl = rememberedWebUiUrl();
        }
        final String probeUrl = rememberedUrl;
        final boolean enableWebButtons = rememberedUrl != null && !rememberedUrl.isEmpty();
        final long probeSerial = cancelClassicFailureProbe();
        runOnUiThread(() -> {
            sendButton.setEnabled(false);
            readStatusButton.setEnabled(false);
            disconnectButton.setEnabled(false);
            setWebButtonsEnabled(enableWebButtons);
            deviceInfoView.setText("Device info: not connected");
            refreshConnectButtonState();
        });
        if (probeUrl != null && !probeUrl.isEmpty()) {
            probeRememberedWebUiAsync(probeUrl, probeSerial);
        }
    }

    private void clearConnectivityDiagnostics() {
        lastWebUiProbeSummary = "";
        lastSubnetScanSummary = "";
    }

    private void probeRememberedWebUiAsync(String baseUrl, long probeSerial) {
        String healthzUrl = normalizeHealthzUrl(baseUrl);
        String rememberedHost = extractHost(baseUrl);
        new Thread(() -> {
            boolean reachable = isHttpReachable(healthzUrl);
            synchronized (this) {
                if (probeSerial != classicFailureProbeSerial) {
                    return;
                }
            }
            String currentStatus = lastStatusText;
            if (!isClassicConnectFailureMessage(currentStatus)) {
                return;
            }
            if (reachable) {
                lastWebUiProbeSummary = "last known WebUI reachable from phone";
                if (!currentStatus.contains("OPEN WEBUI")) {
                    setStatus(currentStatus + " Last known WebUI still responds from this phone.");
                }
                return;
            }
            lastWebUiProbeSummary = "last known WebUI unreachable from phone";
            if (!currentStatus.contains("Last known WebUI is unreachable")) {
                setStatus(currentStatus + " Last known WebUI is unreachable from this phone right now, so T4 may have lost Wi-Fi or changed IP.");
            }
            Ipv4Network.AddressInfo phoneInfo = Ipv4Network.currentAddressInfo(
                    getSystemService(ConnectivityManager.class)
            );
            if (phoneInfo != null && phoneInfo.prefixLength == 24) {
                scanCurrentSubnetForLumeloWebUiAsync(phoneInfo, rememberedHost, probeSerial);
            } else {
                lastSubnetScanSummary = "skipped current-subnet scan (phone IPv4 is not /24)";
            }
        }, "LumeloWebUiProbe").start();
    }

    private void scanCurrentSubnetForLumeloWebUiAsync(
            Ipv4Network.AddressInfo phoneInfo,
            String rememberedHost,
            long probeSerial
    ) {
        String subnetPrefix = subnet24Prefix(phoneInfo.address);
        if (subnetPrefix.isEmpty()) {
            lastSubnetScanSummary = "skipped current-subnet scan (phone IPv4 subnet unavailable)";
            return;
        }
        lastSubnetScanSummary = "scanning current /24 subnet for another Lumelo WebUI";
        if (!lastStatusText.contains("Scanning current /24 subnet")) {
            setStatus(lastStatusText + " Scanning current /24 subnet for another Lumelo WebUI...");
        }
        new Thread(() -> {
            ExecutorService executor = Executors.newFixedThreadPool(WEBUI_SUBNET_SCAN_WORKERS);
            ExecutorCompletionService<String> completion = new ExecutorCompletionService<>(executor);
            int taskCount = 0;
            try {
                for (int host = 1; host <= 254; host++) {
                    String candidateHost = subnetPrefix + "." + host;
                    if (candidateHost.equals(phoneInfo.address) || candidateHost.equals(rememberedHost)) {
                        continue;
                    }
                    completion.submit(() -> probeLumeloHealthzHost(candidateHost) ? candidateHost : null);
                    taskCount += 1;
                }
                String foundHost = null;
                for (int index = 0; index < taskCount; index++) {
                    Future<String> result = completion.take();
                    synchronized (this) {
                        if (probeSerial != classicFailureProbeSerial) {
                            executor.shutdownNow();
                            return;
                        }
                    }
                    String candidate = result.get();
                    if (candidate != null) {
                        foundHost = candidate;
                        break;
                    }
                }
                executor.shutdownNow();
                synchronized (this) {
                    if (probeSerial != classicFailureProbeSerial) {
                        return;
                    }
                }
                if (foundHost == null) {
                    lastSubnetScanSummary = "no Lumelo WebUI responded on current /24 subnet";
                    if (!lastStatusText.contains("No Lumelo WebUI responded on the current /24 subnet")) {
                        setStatus(lastStatusText + " No Lumelo WebUI responded on the current /24 subnet.");
                    }
                    return;
                }
                String foundUrl = "http://" + foundHost + "/";
                lastSubnetScanSummary = "found Lumelo WebUI at " + foundUrl;
                rememberReportedWebUiUrl(foundUrl);
                if (!lastStatusText.contains(foundUrl)) {
                    setStatus(lastStatusText + " Found Lumelo WebUI at " + foundUrl + " You can OPEN WEBUI now.");
                }
            } catch (Exception exception) {
                executor.shutdownNow();
                if (exception instanceof InterruptedException) {
                    Thread.currentThread().interrupt();
                }
                synchronized (this) {
                    if (probeSerial != classicFailureProbeSerial) {
                        return;
                    }
                }
                lastSubnetScanSummary = "current-subnet scan failed: " + exception.getClass().getSimpleName();
                logEvent("Current-subnet scan failed: " + exception.getClass().getSimpleName());
                if (!lastStatusText.contains("Current-subnet scan failed")) {
                    setStatus(lastStatusText + " Current-subnet scan failed: " + exception.getClass().getSimpleName() + ".");
                }
            }
        }, "LumeloSubnetScan").start();
    }

    private String subnet24Prefix(String address) {
        if (address == null || address.isEmpty()) {
            return "";
        }
        String[] parts = address.split("\\.");
        if (parts.length != 4) {
            return "";
        }
        return parts[0] + "." + parts[1] + "." + parts[2];
    }

    private boolean probeLumeloHealthzHost(String host) {
        return probeLumeloHealthzUrl("http://" + host + "/healthz", WEBUI_SUBNET_SCAN_TIMEOUT_MS);
    }

    private boolean probeLumeloHealthzUrl(String url, int timeoutMs) {
        HttpURLConnection connection = null;
        try {
            connection = (HttpURLConnection) new URL(url).openConnection();
            connection.setRequestMethod("GET");
            connection.setConnectTimeout(timeoutMs);
            connection.setReadTimeout(timeoutMs);
            connection.setUseCaches(false);
            connection.connect();
            int code = connection.getResponseCode();
            if (code < 200 || code >= 300) {
                return false;
            }
            String body = readResponseBody(connection);
            return body.contains("\"status\":\"ok\"")
                    && body.contains("\"provisioning_available\":true");
        } catch (IOException exception) {
            return false;
        } finally {
            if (connection != null) {
                connection.disconnect();
            }
        }
    }

    private String readResponseBody(HttpURLConnection connection) throws IOException {
        InputStream stream = connection.getInputStream();
        try (InputStream input = stream; ByteArrayOutputStream output = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[1024];
            int read;
            while ((read = input.read(buffer)) != -1) {
                output.write(buffer, 0, read);
            }
            return output.toString(StandardCharsets.UTF_8.name());
        }
    }

    private String normalizeHealthzUrl(String baseUrl) {
        if (baseUrl == null || baseUrl.isEmpty()) {
            return "";
        }
        String normalized = baseUrl.endsWith("/") ? baseUrl.substring(0, baseUrl.length() - 1) : baseUrl;
        if (normalized.endsWith("/healthz")) {
            return normalized;
        }
        return normalized + "/healthz";
    }

    private boolean isHttpReachable(String url) {
        HttpURLConnection connection = null;
        try {
            connection = (HttpURLConnection) new URL(url).openConnection();
            connection.setRequestMethod("GET");
            connection.setConnectTimeout(WEBUI_PROBE_TIMEOUT_MS);
            connection.setReadTimeout(WEBUI_PROBE_TIMEOUT_MS);
            connection.setUseCaches(false);
            connection.connect();
            int code = connection.getResponseCode();
            return code >= 200 && code < 500;
        } catch (IOException exception) {
            return false;
        } finally {
            if (connection != null) {
                connection.disconnect();
            }
        }
    }

    private SharedPreferences prefs() {
        return getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
    }

    private boolean shouldAutoOpenWebUi() {
        String targetHost = extractHost(webUrl);
        if (targetHost.isEmpty() || !isPrivateIpv4(targetHost)) {
            return true;
        }
        Ipv4Network.AddressInfo phoneInfo = Ipv4Network.currentAddressInfo(
                getSystemService(ConnectivityManager.class)
        );
        if (phoneInfo == null) {
            return false;
        }
        return Ipv4Network.sameSubnet(phoneInfo, targetHost);
    }

    private String crossNetworkHint() {
        String targetHost = extractHost(webUrl);
        Ipv4Network.AddressInfo phoneInfo = Ipv4Network.currentAddressInfo(
                getSystemService(ConnectivityManager.class)
        );
        if (targetHost.isEmpty() || phoneInfo == null) {
            return "";
        }
        if (!isPrivateIpv4(targetHost) || Ipv4Network.sameSubnet(phoneInfo, targetHost)) {
            return "";
        }
        return "Phone IP " + phoneInfo.address + " is not on the same local subnet as T4 " + targetHost
                + ". Connect this phone to the same hotspot or router before opening WebUI.";
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
        return Ipv4Network.isPrivateIpv4(ip);
    }

    private boolean isValidWpaPassword(String password) {
        if (password.length() >= 8 && password.length() <= 63) {
            return true;
        }
        return password.length() == 64 && isHex(password);
    }

    private boolean isHex(String value) {
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            boolean isDigit = ch >= '0' && ch <= '9';
            boolean isLowerHex = ch >= 'a' && ch <= 'f';
            boolean isUpperHex = ch >= 'A' && ch <= 'F';
            if (!isDigit && !isLowerHex && !isUpperHex) {
                return false;
            }
        }
        return true;
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
        if (bluetoothGatt == null) {
            return false;
        }
        try {
            return bluetoothGatt.requestMtu(DESIRED_ATT_MTU);
        } catch (SecurityException exception) {
            logEvent("MTU request blocked: " + exception.getClass().getSimpleName());
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
        final int rssi;
        final String listEntry;
        final String detailText;
        final String logLine;

        ScanObservation(
                String address,
                boolean uuidMatch,
                boolean nameMatch,
                boolean bonded,
                boolean remembered,
                int rssi,
                String listEntry,
                String detailText,
                String logLine
        ) {
            this.address = address;
            this.uuidMatch = uuidMatch;
            this.nameMatch = nameMatch;
            this.bonded = bonded;
            this.remembered = remembered;
            this.rssi = rssi;
            this.listEntry = listEntry;
            this.detailText = detailText;
            this.logLine = logLine;
        }
    }
}
