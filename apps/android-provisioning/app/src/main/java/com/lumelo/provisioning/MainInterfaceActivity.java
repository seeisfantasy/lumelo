package com.lumelo.provisioning;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.content.Intent;
import android.graphics.Bitmap;
import android.net.ConnectivityManager;
import android.net.LinkAddress;
import android.net.LinkProperties;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.ViewGroup;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.net.Inet4Address;
import java.net.InetAddress;
import java.util.List;

public class MainInterfaceActivity extends Activity {
    public static final String EXTRA_INITIAL_URL = "com.lumelo.provisioning.extra.INITIAL_URL";

    private WebView webView;
    private TextView statusView;
    private String baseUrl = "";
    private String currentUrl = "";
    private final Handler handler = new Handler(Looper.getMainLooper());
    private ConnectivityManager connectivityManager;
    private ConnectivityManager.NetworkCallback networkCallback;
    private boolean mainFrameLoadFailed;
    private boolean mainFrameRequestFailed;
    private boolean reloadScheduled;
    private String lastErrorDescription = "";
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
            initialUrl = "http://127.0.0.1:18080/";
        }
        baseUrl = normalizeBaseUrl(initialUrl);
        currentUrl = initialUrl;
        connectivityManager = getSystemService(ConnectivityManager.class);
        buildUi();
        registerNetworkCallback();
        loadUrl(initialUrl);
    }

    @Override
    protected void onDestroy() {
        handler.removeCallbacks(reloadRunnable);
        handler.removeCallbacks(failedNetworkPollRunnable);
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
    public void onBackPressed() {
        if (webView != null && webView.canGoBack()) {
            webView.goBack();
            return;
        }
        super.onBackPressed();
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
        statusView.setText("Opening Lumelo main interface...");
        statusView.setTextSize(15);
        statusView.setTextColor(0xff6f675e);
        statusView.setPadding(0, dp(6), 0, dp(10));
        root.addView(statusView, matchWrap());

        LinearLayout primaryActions = actionRow();
        primaryActions.addView(navButton("Home", "/"));
        primaryActions.addView(navButton("Library", "/library"));
        primaryActions.addView(navButton("Logs", "/logs"));
        root.addView(primaryActions, matchWrap());

        LinearLayout secondaryActions = actionRow();
        secondaryActions.addView(navButton("Provisioning", "/provisioning"));
        secondaryActions.addView(actionButton("Browser", this::openInBrowser));
        secondaryActions.addView(actionButton("Setup", this::finish));
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
                statusView.setText("Loading " + url);
            }

            @Override
            public void onPageFinished(WebView view, String url) {
                currentUrl = url;
                if (mainFrameRequestFailed) {
                    statusView.setText(
                            "Main interface load failed: "
                                    + lastErrorDescription
                                    + ". The page will retry automatically after network changes."
                    );
                    return;
                }
                mainFrameLoadFailed = false;
                lastErrorDescription = "";
                handler.removeCallbacks(failedNetworkPollRunnable);
                statusView.setText("Viewing " + url);
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
        } catch (RuntimeException ignored) {
            networkCallback = null;
        }
    }

    private void unregisterNetworkCallback() {
        if (connectivityManager == null || networkCallback == null) {
            return;
        }
        try {
            connectivityManager.unregisterNetworkCallback(networkCallback);
        } catch (RuntimeException ignored) {
            // Ignore callback teardown races during activity shutdown.
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
        statusView.setText("Network changed. Retrying " + currentUrl + "...");
        handler.removeCallbacks(reloadRunnable);
        handler.postDelayed(reloadRunnable, 1200);
    }

    private void startFailedNetworkPolling() {
        handler.removeCallbacks(failedNetworkPollRunnable);
        handler.postDelayed(failedNetworkPollRunnable, 2500);
    }

    private boolean canAttemptReload(String url) {
        String targetHost = extractHost(url);
        String phoneIp = currentIpv4Address();
        if (targetHost.isEmpty() || phoneIp.isEmpty()) {
            return false;
        }
        if (isPrivateIpv4(targetHost)) {
            return sameSubnet(phoneIp, targetHost);
        }
        return true;
    }

    private String buildNetworkErrorMessage(String url, String errorDescription) {
        StringBuilder builder = new StringBuilder("Main interface load failed: ");
        builder.append(errorDescription);

        String targetHost = extractHost(url);
        String phoneIp = currentIpv4Address();
        if (targetHost.isEmpty()) {
            builder.append(". The page will retry automatically after network changes.");
            return builder.toString();
        }

        if (phoneIp.isEmpty()) {
            builder.append(". Phone currently has no active IPv4 network. The page will retry automatically after network changes.");
            return builder.toString();
        }

        if (isPrivateIpv4(targetHost) && !sameSubnet(phoneIp, targetHost)) {
            builder.append(". Phone IP ");
            builder.append(phoneIp);
            builder.append(" is not on the same local subnet as T4 ");
            builder.append(targetHost);
            builder.append(". Switch the phone to the same hotspot or router; this page will retry automatically.");
            return builder.toString();
        }

        builder.append(". Phone IP ");
        builder.append(phoneIp);
        builder.append(" can see a network, so this page will retry automatically after connectivity changes.");
        return builder.toString();
    }

    private String extractHost(String url) {
        if (url == null || url.isEmpty()) {
            return "";
        }
        Uri uri = Uri.parse(url);
        return uri.getHost() == null ? "" : uri.getHost();
    }

    private String currentIpv4Address() {
        if (connectivityManager == null) {
            return "";
        }
        Network activeNetwork = connectivityManager.getActiveNetwork();
        if (activeNetwork == null) {
            return "";
        }
        LinkProperties properties = connectivityManager.getLinkProperties(activeNetwork);
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
