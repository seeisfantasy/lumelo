package com.lumelo.provisioning;

import android.Manifest;
import android.annotation.SuppressLint;
import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.net.ConnectivityManager;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.Uri;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.view.ViewGroup;
import android.window.OnBackInvokedCallback;
import android.window.OnBackInvokedDispatcher;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

public class MainInterfaceActivity extends Activity {
    private static final String TAG = "LumeloWebView";
    public static final String EXTRA_INITIAL_URL = "com.lumelo.provisioning.extra.INITIAL_URL";
    public static final String EXTRA_EXPECTED_T4_SSID = "com.lumelo.provisioning.extra.EXPECTED_T4_SSID";

    private WebView webView;
    private TextView statusView;
    private String baseUrl = "";
    private String currentUrl = "";
    private String expectedT4Ssid = "";
    private final Handler handler = new Handler(Looper.getMainLooper());
    private ConnectivityManager connectivityManager;
    private ConnectivityManager.NetworkCallback networkCallback;
    private boolean mainFrameLoadFailed;
    private boolean mainFrameRequestFailed;
    private boolean reloadScheduled;
    private String lastErrorDescription = "";
    private OnBackInvokedCallback backInvokedCallback;
    private final Runnable failedNetworkPollRunnable = new Runnable() {
        @Override
        public void run() {
            if (!mainFrameLoadFailed || webView == null || currentUrl == null || currentUrl.isEmpty()) {
                return;
            }
            statusView.setText(buildNetworkErrorMessage(currentUrl, lastErrorDescription));
            if (!reloadScheduled && canAttemptReload(currentUrl)) {
                scheduleRecoveryReload("periodic network check");
            }
            handler.postDelayed(this, 2500);
        }
    };
    private final Runnable reloadRunnable = new Runnable() {
        @Override
        public void run() {
            reloadScheduled = false;
            if (!mainFrameLoadFailed || webView == null || currentUrl == null || currentUrl.isEmpty()) {
                return;
            }
            statusView.setText("Network changed. Retrying " + currentUrl);
            webView.loadUrl(currentUrl);
        }
    };

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        String initialUrl = getIntent().getStringExtra(EXTRA_INITIAL_URL);
        if (initialUrl == null || initialUrl.isEmpty()) {
            initialUrl = "http://127.0.0.1/";
        }
        baseUrl = normalizeBaseUrl(initialUrl);
        currentUrl = initialUrl;
        expectedT4Ssid = normalizeSsid(getIntent().getStringExtra(EXTRA_EXPECTED_T4_SSID));
        connectivityManager = getSystemService(ConnectivityManager.class);
        buildUi();
        registerBackNavigationCallback();
        registerNetworkCallback();
        loadUrl(initialUrl);
    }

    @Override
    protected void onDestroy() {
        handler.removeCallbacks(reloadRunnable);
        handler.removeCallbacks(failedNetworkPollRunnable);
        unregisterBackNavigationCallback();
        unregisterNetworkCallback();
        if (webView != null) {
            webView.destroy();
            webView = null;
        }
        super.onDestroy();
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (mainFrameLoadFailed) {
            startFailedNetworkPolling();
            scheduleRecoveryReload("activity resumed");
        }
    }

    @Override
    @SuppressLint("GestureBackNavigation")
    public void onBackPressed() {
        handleBackNavigation();
    }

    private void handleBackNavigation() {
        if (webView != null && webView.canGoBack()) {
            webView.goBack();
            return;
        }
        super.onBackPressed();
    }

    private void registerBackNavigationCallback() {
        if (android.os.Build.VERSION.SDK_INT < android.os.Build.VERSION_CODES.TIRAMISU) {
            return;
        }
        backInvokedCallback = this::handleBackNavigation;
        getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
                backInvokedCallback
        );
    }

    private void unregisterBackNavigationCallback() {
        if (android.os.Build.VERSION.SDK_INT < android.os.Build.VERSION_CODES.TIRAMISU || backInvokedCallback == null) {
            return;
        }
        getOnBackInvokedDispatcher().unregisterOnBackInvokedCallback(backInvokedCallback);
        backInvokedCallback = null;
    }

    @SuppressLint("SetJavaScriptEnabled")
    private void buildUi() {
        int padding = dp(16);
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(padding, padding, padding, padding);
        root.setBackgroundColor(0xfff3efe7);
        setContentView(root);

        TextView title = new TextView(this);
        title.setText("Lumelo");
        title.setTextSize(26);
        title.setTextColor(0xff1f1d1a);
        root.addView(title, matchWrap());

        statusView = new TextView(this);
        statusView.setText("正在打开 Lumelo 主界面...");
        statusView.setTextSize(15);
        statusView.setTextColor(0xff6f675e);
        statusView.setPadding(0, dp(6), 0, dp(10));
        root.addView(statusView, matchWrap());

        LinearLayout primaryActions = actionRow();
        primaryActions.addView(navButton("首页", "/"));
        primaryActions.addView(navButton("曲库", "/library"));
        primaryActions.addView(navButton("日志", "/logs"));
        root.addView(primaryActions, matchWrap());

        LinearLayout secondaryActions = actionRow();
        secondaryActions.addView(navButton("设置", "/provisioning"));
        secondaryActions.addView(actionButton("重试", this::retryCurrentUrl));
        secondaryActions.addView(actionButton("浏览器", this::openInBrowser));
        secondaryActions.addView(actionButton("返回", this::finish));
        root.addView(secondaryActions, matchWrap());

        webView = new WebView(this);
        WebSettings settings = webView.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setSupportZoom(false);
        settings.setBuiltInZoomControls(false);
        settings.setDisplayZoomControls(false);
        settings.setLoadWithOverviewMode(true);
        settings.setUseWideViewPort(true);
        webView.setWebChromeClient(new WebChromeClient());
        webView.setWebViewClient(new WebViewClient() {
            @Override
            public void onPageStarted(WebView view, String url, Bitmap favicon) {
                currentUrl = url;
                mainFrameRequestFailed = false;
                statusView.setText("正在打开 " + url);
            }

            @Override
            public void onPageFinished(WebView view, String url) {
                currentUrl = url;
                if (mainFrameRequestFailed) {
                    statusView.setText(buildNetworkErrorMessage(currentUrl, lastErrorDescription));
                    return;
                }
                mainFrameLoadFailed = false;
                lastErrorDescription = "";
                handler.removeCallbacks(failedNetworkPollRunnable);
                statusView.setText("当前页面 " + url);
            }

            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                return false;
            }

            @Override
            public void onReceivedError(
                    WebView view,
                    WebResourceRequest request,
                    WebResourceError error
            ) {
                if (request != null && request.isForMainFrame() && error != null) {
                    mainFrameLoadFailed = true;
                    mainFrameRequestFailed = true;
                    lastErrorDescription = String.valueOf(error.getDescription());
                    currentUrl = request.getUrl() == null ? currentUrl : request.getUrl().toString();
                    statusView.setText(buildNetworkErrorMessage(currentUrl, lastErrorDescription));
                    startFailedNetworkPolling();
                }
            }
        });
        root.addView(webView, new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                0,
                1f
        ));
    }

    private LinearLayout actionRow() {
        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);
        row.setPadding(0, 0, 0, dp(8));
        return row;
    }

    private Button navButton(String label, String path) {
        return actionButton(label, () -> loadUrl(joinPath(path)));
    }

    private Button actionButton(String label, Runnable action) {
        Button button = new Button(this);
        button.setText(label);
        button.setOnClickListener(view -> action.run());
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
                0,
                ViewGroup.LayoutParams.WRAP_CONTENT,
                1f
        );
        params.setMarginEnd(dp(8));
        button.setLayoutParams(params);
        return button;
    }

    private void loadUrl(String url) {
        currentUrl = url;
        mainFrameRequestFailed = false;
        handler.removeCallbacks(failedNetworkPollRunnable);
        if (webView != null) {
            webView.loadUrl(url);
        }
    }

    private void openInBrowser() {
        if (currentUrl == null || currentUrl.isEmpty()) {
            return;
        }
        startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(currentUrl)));
    }

    private void retryCurrentUrl() {
        if (currentUrl == null || currentUrl.isEmpty()) {
            return;
        }
        reloadScheduled = false;
        mainFrameRequestFailed = false;
        handler.removeCallbacks(reloadRunnable);
        handler.removeCallbacks(failedNetworkPollRunnable);
        statusView.setText("正在重试 " + currentUrl + "...");
        if (webView != null) {
            webView.loadUrl(currentUrl);
        }
    }

    private String joinPath(String path) {
        String normalizedBase = baseUrl.endsWith("/") ? baseUrl.substring(0, baseUrl.length() - 1) : baseUrl;
        if (path == null || path.isEmpty() || "/".equals(path)) {
            return normalizedBase + "/";
        }
        if (path.startsWith("/")) {
            return normalizedBase + path;
        }
        return normalizedBase + "/" + path;
    }

    private String normalizeBaseUrl(String url) {
        Uri uri = Uri.parse(url);
        if (uri.getScheme() == null || uri.getHost() == null) {
            return url;
        }
        StringBuilder builder = new StringBuilder();
        builder.append(uri.getScheme()).append("://").append(uri.getHost());
        if (uri.getPort() != -1) {
            builder.append(":").append(uri.getPort());
        }
        return builder.toString();
    }

    private void registerNetworkCallback() {
        if (connectivityManager == null || networkCallback != null) {
            return;
        }
        networkCallback = new ConnectivityManager.NetworkCallback() {
            @Override
            public void onAvailable(Network network) {
                scheduleRecoveryReload("network available");
            }

            @Override
            public void onCapabilitiesChanged(Network network, NetworkCapabilities capabilities) {
                if (capabilities != null
                        && capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)) {
                    scheduleRecoveryReload("network changed");
                }
            }
        };
        try {
            connectivityManager.registerDefaultNetworkCallback(networkCallback);
        } catch (RuntimeException exception) {
            Log.w(TAG, "Failed to register default network callback", exception);
            networkCallback = null;
        }
    }

    private void unregisterNetworkCallback() {
        if (connectivityManager == null || networkCallback == null) {
            return;
        }
        try {
            connectivityManager.unregisterNetworkCallback(networkCallback);
        } catch (RuntimeException exception) {
            Log.w(TAG, "Failed to unregister default network callback", exception);
        }
        networkCallback = null;
    }

    private void scheduleRecoveryReload(String reason) {
        if (Looper.myLooper() != Looper.getMainLooper()) {
            handler.post(() -> scheduleRecoveryReload(reason));
            return;
        }
        if (!mainFrameLoadFailed || currentUrl == null || currentUrl.isEmpty()) {
            return;
        }
        if (reloadScheduled) {
            return;
        }
        reloadScheduled = true;
        statusView.setText("网络已变化，正在重试 " + currentUrl + "...");
        handler.removeCallbacks(reloadRunnable);
        handler.postDelayed(reloadRunnable, 1200);
    }

    private void startFailedNetworkPolling() {
        handler.removeCallbacks(failedNetworkPollRunnable);
        handler.postDelayed(failedNetworkPollRunnable, 2500);
    }

    private boolean canAttemptReload(String url) {
        String targetHost = extractHost(url);
        Ipv4Network.AddressInfo phoneInfo = Ipv4Network.currentAddressInfo(connectivityManager);
        if (targetHost.isEmpty() || phoneInfo == null) {
            return false;
        }
        if (Ipv4Network.isPrivateIpv4(targetHost)) {
            return Ipv4Network.sameSubnet(phoneInfo, targetHost);
        }
        return true;
    }

    private String buildNetworkErrorMessage(String url, String errorDescription) {
        StringBuilder builder = new StringBuilder("主界面加载失败：");
        builder.append(errorDescription);

        String targetHost = extractHost(url);
        Ipv4Network.AddressInfo phoneInfo = Ipv4Network.currentAddressInfo(connectivityManager);
        String currentWifiSsid = currentConnectedWifiSsid();
        String currentWifiLabel = currentWifiSsid.isEmpty() ? "(unknown)" : currentWifiSsid;
        if (!targetHost.isEmpty()) {
            builder.append("\nT4 WebUI：").append(targetHost);
        }
        if (!expectedT4Ssid.isEmpty()) {
            builder.append("\n上次确认的 T4 Wi-Fi：").append(expectedT4Ssid);
        }
        builder.append("\n手机当前 Wi-Fi：").append(currentWifiLabel);
        if (targetHost.isEmpty()) {
            builder.append("\n恢复建议：等待网络变化后的自动重试，或点击“重试”。");
            return builder.toString();
        }

        if (phoneInfo == null) {
            builder.append("\n手机 IPv4：(none)");
            builder.append("\n恢复建议：让这台手机重新连回 ");
            if (!expectedT4Ssid.isEmpty()) {
                builder.append(expectedT4Ssid);
            } else {
                builder.append("与 T4 相同的热点或路由器");
            }
            builder.append("，然后等待自动重试，或点击“重试”。");
            return builder.toString();
        }

        builder.append("\n手机 IPv4：")
                .append(phoneInfo.address)
                .append("/")
                .append(phoneInfo.prefixLength);

        if (Ipv4Network.isPrivateIpv4(targetHost) && !Ipv4Network.sameSubnet(phoneInfo, targetHost)) {
            builder.append("\n恢复建议：手机当前网络还不能直接访问 T4 ");
            builder.append(targetHost);
            if (!expectedT4Ssid.isEmpty()) {
                builder.append("。请把手机切回 ").append(expectedT4Ssid);
            } else {
                builder.append("。请把手机切回与 T4 相同的热点或路由器");
            }
            builder.append("，然后等待自动重试，或点击“重试”。");
            return builder.toString();
        }

        builder.append("\n恢复建议：手机网络看起来已经可达。等待自动重试，或点击“重试”。");
        return builder.toString();
    }

    private String extractHost(String url) {
        if (url == null || url.isEmpty()) {
            return "";
        }
        Uri uri = Uri.parse(url);
        return uri.getHost() == null ? "" : uri.getHost();
    }

    private String normalizeSsid(String value) {
        if (value == null) {
            return "";
        }
        return value.trim();
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

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
        );
    }

    private int dp(int value) {
        return (int) (value * getResources().getDisplayMetrics().density + 0.5f);
    }
}
