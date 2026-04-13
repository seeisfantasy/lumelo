# AI Review Part 04

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/MainInterfaceActivity.java`

- bytes: 16264
- segment: 1/1

~~~java
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
~~~

## `apps/android-provisioning/app/src/main/java/com/lumelo/provisioning/ProvisioningSecurity.java`

- bytes: 13644
- segment: 1/1

~~~java
package com.lumelo.provisioning;

import java.io.ByteArrayOutputStream;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.security.GeneralSecurityException;
import java.security.SecureRandom;
import java.util.Arrays;
import java.util.Base64;

import javax.crypto.Mac;
import javax.crypto.spec.SecretKeySpec;

final class ProvisioningSecurity {
    static final String CREDENTIAL_SCHEME = "dh-hmac-sha256-stream-v1";

    private static final String DH_GROUP = "modp14-sha256";
    private static final String DH_PRIME_HEX =
            "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD129024E08"
                    + "8A67CC74020BBEA63B139B22514A08798E3404DDEF9519B3CD"
                    + "3A431B302B0A6DF25F14374FE1356D6D51C245E485B576625E"
                    + "7EC6F44C42E9A637ED6B0BFF5CB6F406B7EDEE386BFB5A899F"
                    + "A5AE9F24117C4B1FE649286651ECE45B3DC2007CB8A163BF05"
                    + "98DA48361C55D39A69163FA8FD24CF5F83655D23DCA3AD961C"
                    + "62F356208552BB9ED529077096966D670C354E4ABC9804F174"
                    + "6C08CA18217C32905E462E36CE3BE39E772C180E86039B2783"
                    + "A2EC07A28FB5C55DF06F4C52C9DE2BCBF6955817183995497C"
                    + "EA956AE515D2261898FA051015728E5A8AACAA68FFFFFFFFFF"
                    + "FFFFFFFF";
    private static final BigInteger DH_PRIME = new BigInteger(DH_PRIME_HEX, 16);
    private static final BigInteger DH_GENERATOR = BigInteger.valueOf(2L);
    private static final int DH_PUBLIC_KEY_BYTES = (DH_PRIME.bitLength() + 7) / 8;
    private static final int PRIVATE_KEY_BYTES = 32;
    private static final int NONCE_BYTES = 16;
    private static final int DERIVED_KEY_BYTES = 64;
    private static final byte[] HKDF_SALT_LABEL = ascii("lumelo-bt-session-salt-v1");
    private static final byte[] HKDF_INFO_LABEL = ascii("lumelo-bt-credentials-v1");
    private static final byte[] STREAM_LABEL = ascii("lumelo-bt-stream-v1");
    private static final byte[] MAC_LABEL = ascii("lumelo-bt-mac-v1");
    private static final SecureRandom SECURE_RANDOM = new SecureRandom();

    private ProvisioningSecurity() {
    }

    static Session parseSession(
            String sessionId,
            String scheme,
            String group,
            String serverNonceBase64,
            String serverPublicKeyBase64
    ) throws GeneralSecurityException {
        if (!CREDENTIAL_SCHEME.equals(scheme) || !DH_GROUP.equals(group) || sessionId.isEmpty()) {
            throw new GeneralSecurityException("Unsupported credential security parameters");
        }

        byte[] serverNonce = decodeBase64(serverNonceBase64);
        byte[] serverPublicKey = decodeBase64(serverPublicKeyBase64);
        if (serverNonce.length != NONCE_BYTES || serverPublicKey.length != DH_PUBLIC_KEY_BYTES) {
            throw new GeneralSecurityException("Invalid credential security payload length");
        }

        BigInteger serverPublic = new BigInteger(1, serverPublicKey);
        validatePeerPublicKey(serverPublic);
        return new Session(sessionId, serverNonce, serverPublic);
    }

    static EncryptedPayload encryptCredentials(Session session, String ssid, String password)
            throws GeneralSecurityException {
        if (session == null) {
            throw new GeneralSecurityException("Secure credential transport is unavailable");
        }

        BigInteger privateKey = randomPrivateKey();
        BigInteger clientPublic = DH_GENERATOR.modPow(privateKey, DH_PRIME);
        byte[] clientPublicBytes = toFixedBytes(clientPublic);
        byte[] sharedSecret = toFixedBytes(session.serverPublic.modPow(privateKey, DH_PRIME));
        byte[] clientNonce = randomBytes(NONCE_BYTES);
        byte[] messageNonce = randomBytes(NONCE_BYTES);

        byte[] plaintext = buildCredentialJson(ssid, password).getBytes(StandardCharsets.UTF_8);

        DerivedKeys keys = deriveKeys(sharedSecret, session.sessionId, session.serverNonce, clientNonce);
        byte[] ciphertext = xorWithStream(plaintext, keys.streamKey, messageNonce);
        byte[] mac = computeMac(
                keys.macKey,
                session.sessionId,
                session.serverNonce,
                clientNonce,
                messageNonce,
                clientPublicBytes,
                ciphertext
        );

        return new EncryptedPayload(
                CREDENTIAL_SCHEME,
                DH_GROUP,
                session.sessionId,
                encodeBase64(clientPublicBytes),
                encodeBase64(clientNonce),
                encodeBase64(messageNonce),
                encodeBase64(ciphertext),
                encodeBase64(mac)
        );
    }

    private static DerivedKeys deriveKeys(
            byte[] sharedSecret,
            String sessionId,
            byte[] serverNonce,
            byte[] clientNonce
    ) throws GeneralSecurityException {
        byte[] salt = concat(HKDF_SALT_LABEL, serverNonce, clientNonce);
        byte[] prk = hmacSha256(salt, sharedSecret);
        byte[] okm = hkdfExpand(prk, concat(HKDF_INFO_LABEL, ascii(sessionId)), DERIVED_KEY_BYTES);
        return new DerivedKeys(
                Arrays.copyOfRange(okm, 0, 32),
                Arrays.copyOfRange(okm, 32, 64)
        );
    }

    private static byte[] xorWithStream(byte[] plaintext, byte[] streamKey, byte[] messageNonce)
            throws GeneralSecurityException {
        byte[] output = new byte[plaintext.length];
        int offset = 0;
        int counter = 0;
        while (offset < plaintext.length) {
            byte[] streamBlock = hmacSha256(
                    streamKey,
                    concat(STREAM_LABEL, messageNonce, intToBytes(counter))
            );
            int blockSize = Math.min(streamBlock.length, plaintext.length - offset);
            for (int index = 0; index < blockSize; index++) {
                output[offset + index] = (byte) (plaintext[offset + index] ^ streamBlock[index]);
            }
            offset += blockSize;
            counter += 1;
        }
        return output;
    }

    private static byte[] computeMac(
            byte[] macKey,
            String sessionId,
            byte[] serverNonce,
            byte[] clientNonce,
            byte[] messageNonce,
            byte[] clientPublicKey,
            byte[] ciphertext
    ) throws GeneralSecurityException {
        ByteArrayOutputStream builder = new ByteArrayOutputStream();
        appendLengthPrefixed(builder, MAC_LABEL);
        appendLengthPrefixed(builder, ascii(sessionId));
        appendLengthPrefixed(builder, serverNonce);
        appendLengthPrefixed(builder, clientNonce);
        appendLengthPrefixed(builder, messageNonce);
        appendLengthPrefixed(builder, clientPublicKey);
        appendLengthPrefixed(builder, ciphertext);
        return hmacSha256(macKey, builder.toByteArray());
    }

    private static String buildCredentialJson(String ssid, String password) {
        return "{\"ssid\":\"" + escapeJson(ssid) + "\",\"password\":\"" + escapeJson(password) + "\"}";
    }

    private static String escapeJson(String value) {
        StringBuilder escaped = new StringBuilder(value.length() + 16);
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            switch (ch) {
                case '\\':
                    escaped.append("\\\\");
                    break;
                case '"':
                    escaped.append("\\\"");
                    break;
                case '\b':
                    escaped.append("\\b");
                    break;
                case '\f':
                    escaped.append("\\f");
                    break;
                case '\n':
                    escaped.append("\\n");
                    break;
                case '\r':
                    escaped.append("\\r");
                    break;
                case '\t':
                    escaped.append("\\t");
                    break;
                default:
                    if (ch < 0x20) {
                        escaped.append(String.format("\\u%04x", (int) ch));
                    } else {
                        escaped.append(ch);
                    }
                    break;
            }
        }
        return escaped.toString();
    }

    private static void appendLengthPrefixed(ByteArrayOutputStream builder, byte[] value) {
        byte[] lengthPrefix = intToBytes(value.length);
        builder.write(lengthPrefix, 0, lengthPrefix.length);
        builder.write(value, 0, value.length);
    }

    private static BigInteger randomPrivateKey() {
        while (true) {
            BigInteger candidate = new BigInteger(1, randomBytes(PRIVATE_KEY_BYTES));
            if (candidate.compareTo(BigInteger.TWO) >= 0
                    && candidate.compareTo(DH_PRIME.subtract(BigInteger.TWO)) <= 0) {
                return candidate;
            }
        }
    }

    private static void validatePeerPublicKey(BigInteger value) throws GeneralSecurityException {
        if (value.compareTo(BigInteger.TWO) < 0
                || value.compareTo(DH_PRIME.subtract(BigInteger.TWO)) > 0) {
            throw new GeneralSecurityException("Peer public key is outside the allowed range");
        }
    }

    private static byte[] toFixedBytes(BigInteger value) {
        byte[] raw = value.toByteArray();
        if (raw.length == DH_PUBLIC_KEY_BYTES) {
            return raw;
        }
        if (raw.length == DH_PUBLIC_KEY_BYTES + 1 && raw[0] == 0) {
            return Arrays.copyOfRange(raw, 1, raw.length);
        }
        byte[] fixed = new byte[DH_PUBLIC_KEY_BYTES];
        int copyOffset = Math.max(0, raw.length - DH_PUBLIC_KEY_BYTES);
        int copyLength = Math.min(raw.length, DH_PUBLIC_KEY_BYTES);
        System.arraycopy(raw, copyOffset, fixed, DH_PUBLIC_KEY_BYTES - copyLength, copyLength);
        return fixed;
    }

    private static byte[] hkdfExpand(byte[] prk, byte[] info, int outputLength)
            throws GeneralSecurityException {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        byte[] previous = new byte[0];
        int counter = 1;
        while (output.size() < outputLength) {
            byte[] blockInput = concat(previous, info, new byte[]{(byte) counter});
            previous = hmacSha256(prk, blockInput);
            output.write(previous, 0, previous.length);
            counter += 1;
        }
        return Arrays.copyOf(output.toByteArray(), outputLength);
    }

    private static byte[] hmacSha256(byte[] key, byte[] message) throws GeneralSecurityException {
        Mac mac = Mac.getInstance("HmacSHA256");
        mac.init(new SecretKeySpec(key, "HmacSHA256"));
        return mac.doFinal(message);
    }

    private static String encodeBase64(byte[] value) {
        return Base64.getEncoder().encodeToString(value);
    }

    private static byte[] decodeBase64(String value) {
        return Base64.getDecoder().decode(value);
    }

    private static byte[] ascii(String value) {
        return value.getBytes(StandardCharsets.US_ASCII);
    }

    private static byte[] randomBytes(int count) {
        byte[] value = new byte[count];
        SECURE_RANDOM.nextBytes(value);
        return value;
    }

    private static byte[] intToBytes(int value) {
        return new byte[]{
                (byte) ((value >>> 24) & 0xff),
                (byte) ((value >>> 16) & 0xff),
                (byte) ((value >>> 8) & 0xff),
                (byte) (value & 0xff)
        };
    }

    private static byte[] concat(byte[]... values) {
        int totalLength = 0;
        for (byte[] value : values) {
            totalLength += value.length;
        }
        byte[] output = new byte[totalLength];
        int offset = 0;
        for (byte[] value : values) {
            System.arraycopy(value, 0, output, offset, value.length);
            offset += value.length;
        }
        return output;
    }

    static final class Session {
        final String sessionId;
        final byte[] serverNonce;
        final BigInteger serverPublic;

        Session(String sessionId, byte[] serverNonce, BigInteger serverPublic) {
            this.sessionId = sessionId;
            this.serverNonce = Arrays.copyOf(serverNonce, serverNonce.length);
            this.serverPublic = serverPublic;
        }
    }

    static final class EncryptedPayload {
        final String scheme;
        final String dhGroup;
        final String sessionId;
        final String clientPublicKey;
        final String clientNonce;
        final String messageNonce;
        final String ciphertext;
        final String mac;

        EncryptedPayload(
                String scheme,
                String dhGroup,
                String sessionId,
                String clientPublicKey,
                String clientNonce,
                String messageNonce,
                String ciphertext,
                String mac
        ) {
            this.scheme = scheme;
            this.dhGroup = dhGroup;
            this.sessionId = sessionId;
            this.clientPublicKey = clientPublicKey;
            this.clientNonce = clientNonce;
            this.messageNonce = messageNonce;
            this.ciphertext = ciphertext;
            this.mac = mac;
        }
    }

    private static final class DerivedKeys {
        final byte[] streamKey;
        final byte[] macKey;

        DerivedKeys(byte[] streamKey, byte[] macKey) {
            this.streamKey = streamKey;
            this.macKey = macKey;
        }
    }
}
~~~

## `apps/android-provisioning/app/src/main/res/values/styles.xml`

- bytes: 358
- segment: 1/1

~~~xml
<resources>
    <style name="AppTheme" parent="android:style/Theme.Material.Light.NoActionBar">
        <item name="android:fontFamily">sans</item>
        <item name="android:windowLightStatusBar">true</item>
        <item name="android:navigationBarColor">#f3efe7</item>
        <item name="android:statusBarColor">#f3efe7</item>
    </style>
</resources>
~~~

## `apps/android-provisioning/build.gradle.kts`

- bytes: 725
- segment: 1/1

~~~kotlin
import java.nio.file.Files
import java.util.Comparator

plugins {
    id("com.android.application") version "8.13.2" apply false
}

val cleanAppleDouble by tasks.registering {
    doNotTrackState("Always clean AppleDouble sidecar files before Android builds on external volumes.")
    doLast {
        Files.walk(projectDir.toPath()).use { paths ->
            paths
                .filter { path -> path.fileName.toString().startsWith("._") }
                .sorted(Comparator.reverseOrder())
                .forEach { path -> Files.deleteIfExists(path) }
        }
    }
}

subprojects {
    tasks.matching { it.name == "preBuild" }.configureEach {
        dependsOn(rootProject.tasks.named("cleanAppleDouble"))
    }
}
~~~

## `apps/android-provisioning/gradle/wrapper/gradle-wrapper.properties`

- bytes: 251
- segment: 1/1

~~~properties
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-8.13-bin.zip
networkTimeout=10000
validateDistributionUrl=true
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
~~~

## `apps/android-provisioning/gradlew`

- bytes: 8752
- segment: 1/1

~~~bash
#!/bin/sh

#
# Copyright © 2015-2021 the original authors.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0
#

##############################################################################
#
#   Gradle start up script for POSIX generated by Gradle.
#
#   Important for running:
#
#   (1) You need a POSIX-compliant shell to run this script. If your /bin/sh is
#       noncompliant, but you have some other compliant shell such as ksh or
#       bash, then to run this script, type that shell name before the whole
#       command line, like:
#
#           ksh Gradle
#
#       Busybox and similar reduced shells will NOT work, because this script
#       requires all of these POSIX shell features:
#         * functions;
#         * expansions «$var», «${var}», «${var:-default}», «${var+SET}»,
#           «${var#prefix}», «${var%suffix}», and «$( cmd )»;
#         * compound commands having a testable exit status, especially «case»;
#         * various built-in commands including «command», «set», and «ulimit».
#
#   Important for patching:
#
#   (2) This script targets any POSIX shell, so it avoids extensions provided
#       by Bash, Ksh, etc; in particular arrays are avoided.
#
#       The "traditional" practice of packing multiple parameters into a
#       space-separated string is a well documented source of bugs and security
#       problems, so this is (mostly) avoided, by progressively accumulating
#       options in "$@", and eventually passing that to Java.
#
#       Where the inherited environment variables (DEFAULT_JVM_OPTS, JAVA_OPTS,
#       and GRADLE_OPTS) rely on word-splitting, this is performed explicitly;
#       see the in-line comments for details.
#
#       There are tweaks for specific operating systems such as AIX, CygWin,
#       Darwin, MinGW, and NonStop.
#
#   (3) This script is generated from the Groovy template
#       https://github.com/gradle/gradle/blob/HEAD/platforms/jvm/plugins-application/src/main/resources/org/gradle/api/internal/plugins/unixStartScript.txt
#       within the Gradle project.
#
#       You can find Gradle at https://github.com/gradle/gradle/.
#
##############################################################################

# Attempt to set APP_HOME

# Resolve links: $0 may be a link
app_path=$0

# Need this for daisy-chained symlinks.
while
    APP_HOME=${app_path%"${app_path##*/}"}  # leaves a trailing /; empty if no leading path
    [ -h "$app_path" ]
do
    ls=$( ls -ld "$app_path" )
    link=${ls#*' -> '}
    case $link in             #(
      /*)   app_path=$link ;; #(
      *)    app_path=$APP_HOME$link ;;
    esac
done

# This is normally unused
# shellcheck disable=SC2034
APP_BASE_NAME=${0##*/}
# Discard cd standard output in case $CDPATH is set (https://github.com/gradle/gradle/issues/25036)
APP_HOME=$( cd -P "${APP_HOME:-./}" > /dev/null && printf '%s\n' "$PWD" ) || exit

# Use the maximum available, or set MAX_FD != -1 to use that value.
MAX_FD=maximum

warn () {
    echo "$*"
} >&2

die () {
    echo
    echo "$*"
    echo
    exit 1
} >&2

# OS specific support (must be 'true' or 'false').
cygwin=false
msys=false
darwin=false
nonstop=false
case "$( uname )" in                #(
  CYGWIN* )         cygwin=true  ;; #(
  Darwin* )         darwin=true  ;; #(
  MSYS* | MINGW* )  msys=true    ;; #(
  NONSTOP* )        nonstop=true ;;
esac

CLASSPATH=$APP_HOME/gradle/wrapper/gradle-wrapper.jar


# Determine the Java command to use to start the JVM.
if [ -n "$JAVA_HOME" ] ; then
    if [ -x "$JAVA_HOME/jre/sh/java" ] ; then
        # IBM's JDK on AIX uses strange locations for the executables
        JAVACMD=$JAVA_HOME/jre/sh/java
    else
        JAVACMD=$JAVA_HOME/bin/java
    fi
    if [ ! -x "$JAVACMD" ] ; then
        die "ERROR: JAVA_HOME is set to an invalid directory: $JAVA_HOME

Please set the JAVA_HOME variable in your environment to match the
location of your Java installation."
    fi
else
    JAVACMD=java
    if ! command -v java >/dev/null 2>&1
    then
        die "ERROR: JAVA_HOME is not set and no 'java' command could be found in your PATH.

Please set the JAVA_HOME variable in your environment to match the
location of your Java installation."
    fi
fi

# Increase the maximum file descriptors if we can.
if ! "$cygwin" && ! "$darwin" && ! "$nonstop" ; then
    case $MAX_FD in #(
      max*)
        # In POSIX sh, ulimit -H is undefined. That's why the result is checked to see if it worked.
        # shellcheck disable=SC2039,SC3045
        MAX_FD=$( ulimit -H -n ) ||
            warn "Could not query maximum file descriptor limit"
    esac
    case $MAX_FD in  #(
      '' | soft) :;; #(
      *)
        # In POSIX sh, ulimit -n is undefined. That's why the result is checked to see if it worked.
        # shellcheck disable=SC2039,SC3045
        ulimit -n "$MAX_FD" ||
            warn "Could not set maximum file descriptor limit to $MAX_FD"
    esac
fi

# Collect all arguments for the java command, stacking in reverse order:
#   * args from the command line
#   * the main class name
#   * -classpath
#   * -D...appname settings
#   * --module-path (only if needed)
#   * DEFAULT_JVM_OPTS, JAVA_OPTS, and GRADLE_OPTS environment variables.

# For Cygwin or MSYS, switch paths to Windows format before running java
if "$cygwin" || "$msys" ; then
    APP_HOME=$( cygpath --path --mixed "$APP_HOME" )
    CLASSPATH=$( cygpath --path --mixed "$CLASSPATH" )

    JAVACMD=$( cygpath --unix "$JAVACMD" )

    # Now convert the arguments - kludge to limit ourselves to /bin/sh
    for arg do
        if
            case $arg in                                #(
              -*)   false ;;                            # don't mess with options #(
              /?*)  t=${arg#/} t=/${t%%/*}              # looks like a POSIX filepath
                    [ -e "$t" ] ;;                      #(
              *)    false ;;
            esac
        then
            arg=$( cygpath --path --ignore --mixed "$arg" )
        fi
        # Roll the args list around exactly as many times as the number of
        # args, so each arg winds up back in the position where it started, but
        # possibly modified.
        #
        # NB: a `for` loop captures its iteration list before it begins, so
        # changing the positional parameters here affects neither the number of
        # iterations, nor the values presented in `arg`.
        shift                   # remove old arg
        set -- "$@" "$arg"      # push replacement arg
    done
fi


# Add default JVM options here. You can also use JAVA_OPTS and GRADLE_OPTS to pass JVM options to this script.
DEFAULT_JVM_OPTS='"-Xmx64m" "-Xms64m"'

# Collect all arguments for the java command:
#   * DEFAULT_JVM_OPTS, JAVA_OPTS, and optsEnvironmentVar are not allowed to contain shell fragments,
#     and any embedded shellness will be escaped.
#   * For example: A user cannot expect ${Hostname} to be expanded, as it is an environment variable and will be
#     treated as '${Hostname}' itself on the command line.

set -- \
        "-Dorg.gradle.appname=$APP_BASE_NAME" \
        -classpath "$CLASSPATH" \
        org.gradle.wrapper.GradleWrapperMain \
        "$@"

# Stop when "xargs" is not available.
if ! command -v xargs >/dev/null 2>&1
then
    die "xargs is not available"
fi

# Use "xargs" to parse quoted args.
#
# With -n1 it outputs one arg per line, with the quotes and backslashes removed.
#
# In Bash we could simply go:
#
#   readarray ARGS < <( xargs -n1 <<<"$var" ) &&
#   set -- "${ARGS[@]}" "$@"
#
# but POSIX shell has neither arrays nor command substitution, so instead we
# post-process each arg (as a line of input to sed) to backslash-escape any
# character that might be a shell metacharacter, then use eval to reverse
# that process (while maintaining the separation between arguments), and wrap
# the whole thing up as a single "set" statement.
#
# This will of course break if any of these variables contains a newline or
# an unmatched quote.
#

eval "set -- $(
        printf '%s\n' "$DEFAULT_JVM_OPTS $JAVA_OPTS $GRADLE_OPTS" |
        xargs -n1 |
        sed ' s~[^-[:alnum:]+,./:=@_]~\\&~g; ' |
        tr '\n' ' '
    )" '"$@"'

exec "$JAVACMD" "$@"
~~~

## `apps/android-provisioning/gradlew.bat`

- bytes: 2966
- segment: 1/1

~~~text
@rem
@rem Copyright 2015 the original author or authors.
@rem
@rem Licensed under the Apache License, Version 2.0 (the "License");
@rem you may not use this file except in compliance with the License.
@rem You may obtain a copy of the License at
@rem
@rem      https://www.apache.org/licenses/LICENSE-2.0
@rem
@rem Unless required by applicable law or agreed to in writing, software
@rem distributed under the License is distributed on an "AS IS" BASIS,
@rem WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
@rem See the License for the specific language governing permissions and
@rem limitations under the License.
@rem
@rem SPDX-License-Identifier: Apache-2.0
@rem

@if "%DEBUG%"=="" @echo off
@rem ##########################################################################
@rem
@rem  Gradle startup script for Windows
@rem
@rem ##########################################################################

@rem Set local scope for the variables with windows NT shell
if "%OS%"=="Windows_NT" setlocal

set DIRNAME=%~dp0
if "%DIRNAME%"=="" set DIRNAME=.
@rem This is normally unused
set APP_BASE_NAME=%~n0
set APP_HOME=%DIRNAME%

@rem Resolve any "." and ".." in APP_HOME to make it shorter.
for %%i in ("%APP_HOME%") do set APP_HOME=%%~fi

@rem Add default JVM options here. You can also use JAVA_OPTS and GRADLE_OPTS to pass JVM options to this script.
set DEFAULT_JVM_OPTS="-Xmx64m" "-Xms64m"

@rem Find java.exe
if defined JAVA_HOME goto findJavaFromJavaHome

set JAVA_EXE=java.exe
%JAVA_EXE% -version >NUL 2>&1
if %ERRORLEVEL% equ 0 goto execute

echo. 1>&2
echo ERROR: JAVA_HOME is not set and no 'java' command could be found in your PATH. 1>&2
echo. 1>&2
echo Please set the JAVA_HOME variable in your environment to match the 1>&2
echo location of your Java installation. 1>&2

goto fail

:findJavaFromJavaHome
set JAVA_HOME=%JAVA_HOME:"=%
set JAVA_EXE=%JAVA_HOME%/bin/java.exe

if exist "%JAVA_EXE%" goto execute

echo. 1>&2
echo ERROR: JAVA_HOME is set to an invalid directory: %JAVA_HOME% 1>&2
echo. 1>&2
echo Please set the JAVA_HOME variable in your environment to match the 1>&2
echo location of your Java installation. 1>&2

goto fail

:execute
@rem Setup the command line

set CLASSPATH=%APP_HOME%\gradle\wrapper\gradle-wrapper.jar


@rem Execute Gradle
"%JAVA_EXE%" %DEFAULT_JVM_OPTS% %JAVA_OPTS% %GRADLE_OPTS% "-Dorg.gradle.appname=%APP_BASE_NAME%" -classpath "%CLASSPATH%" org.gradle.wrapper.GradleWrapperMain %*

:end
@rem End local scope for the variables with windows NT shell
if %ERRORLEVEL% equ 0 goto mainEnd

:fail
rem Set variable GRADLE_EXIT_CONSOLE if you need the _script_ return code instead of
rem the _cmd.exe /c_ return code!
set EXIT_CODE=%ERRORLEVEL%
if %EXIT_CODE% equ 0 set EXIT_CODE=1
if not ""=="%GRADLE_EXIT_CONSOLE%" exit %EXIT_CODE%
exit /b %EXIT_CODE%

:mainEnd
if "%OS%"=="Windows_NT" endlocal

:omega
~~~

## `apps/android-provisioning/settings.gradle.kts`

- bytes: 338
- segment: 1/1

~~~kotlin
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "LumeloProvisioning"
include(":app")
~~~

## `base/README.md`

- bytes: 307
- segment: 1/1

~~~md
# Base Layer

`base/` holds the system-facing part of Lumelo:

- board support placeholders for the FriendlyELEC T4 line
- rootfs overlay files
- systemd units and targets
- boot/build hooks and manifests

This layer should stay focused on image construction and runtime integration,
not application logic.
~~~

## `base/board-support/friendly/README.md`

- bytes: 309
- segment: 1/1

~~~md
# FriendlyELEC Board Support Placeholders

These directories are reserved for the board-level assets we currently plan to
reuse during the first bring-up phase:

- `uboot/`
- `kernel/`
- `dtb/`
- `bootfiles/`

They are intentionally empty until the exact board-support sources and version
pinning are chosen.
~~~

## `base/rootfs/hooks/README.md`

- bytes: 69
- segment: 1/1

~~~md
# Rootfs Hooks

Place image-build and overlay-processing hooks here.
~~~

## `base/rootfs/hooks/t4-bringup-postbuild.sh`

- bytes: 4570
- segment: 1/1

~~~bash
#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: t4-bringup-postbuild.sh <rootfs-dir>" >&2
  exit 1
fi

ROOTFS_DIR=$1
PROFILE=${LUMELO_IMAGE_PROFILE:-t4-bringup}
BOARD_SOURCE_IMAGE=${BOARD_SOURCE_IMAGE:-unknown}
ENABLE_SSH=${ENABLE_SSH:-0}
SSH_AUTHORIZED_KEYS_FILE=${SSH_AUTHORIZED_KEYS_FILE:-}
ROOT_PASSWORD=${ROOT_PASSWORD:-}
ssh_enabled_value=false
root_password_set=0
dev_sshd_config_path="${ROOTFS_DIR}/etc/ssh/sshd_config.d/90-lumelo-development.conf"

unit_dir=
for candidate in /usr/lib/systemd/system /lib/systemd/system; do
  if [ -d "${ROOTFS_DIR}${candidate}" ]; then
    unit_dir=$candidate
    break
  fi
done

mkdir -p "${ROOTFS_DIR}/etc/lumelo"
printf '%s\n' "lumelo" > "${ROOTFS_DIR}/etc/hostname"
cat > "${ROOTFS_DIR}/etc/hosts" <<'EOF'
127.0.0.1 localhost
127.0.1.1 lumelo

::1 localhost ip6-localhost ip6-loopback
ff02::1 ip6-allnodes
ff02::2 ip6-allrouters
EOF

cat > "${ROOTFS_DIR}/etc/fstab" <<'EOF'
# Lumelo-defined rootfs image.
# The board boot chain still provides the kernel command line for root=/dev/mmcblk?p8.
EOF

rm -f "${ROOTFS_DIR}/etc/resolv.conf"
ln -sf /run/systemd/resolve/resolv.conf "${ROOTFS_DIR}/etc/resolv.conf"

mkdir -p "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants"
ln -sf ../local-mode.target \
  "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/local-mode.target"

if [ -n "${unit_dir}" ]; then
  for unit in systemd-networkd.service systemd-resolved.service systemd-timesyncd.service; do
    if [ -f "${ROOTFS_DIR}${unit_dir}/${unit}" ]; then
      ln -sf "${unit_dir}/${unit}" \
        "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/${unit}"
    fi
  done

  if [ -f "${ROOTFS_DIR}/etc/systemd/system/lumelo-bluetooth-uart-attach.service" ]; then
    ln -sf ../lumelo-bluetooth-uart-attach.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/lumelo-bluetooth-uart-attach.service"
  fi

  if [ -f "${ROOTFS_DIR}${unit_dir}/bluetooth.service" ]; then
    ln -sf "${unit_dir}/bluetooth.service" \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/bluetooth.service"
  fi

  if [ -f "${ROOTFS_DIR}/etc/systemd/system/lumelo-bluetooth-provisioning.service" ]; then
    ln -sf ../lumelo-bluetooth-provisioning.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/lumelo-bluetooth-provisioning.service"
  fi

  if [ -f "${ROOTFS_DIR}/etc/systemd/system/lumelo-wifi-provisiond.service" ]; then
    ln -sf ../lumelo-wifi-provisiond.service \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/lumelo-wifi-provisiond.service"
  fi

  if [ "${ENABLE_SSH}" = "1" ] && [ -f "${ROOTFS_DIR}${unit_dir}/ssh.service" ]; then
    ln -sf "${unit_dir}/ssh.service" \
      "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/ssh.service"
    ssh_enabled_value=true

    install -d -m 0755 "$(dirname "${dev_sshd_config_path}")"
    cat > "${dev_sshd_config_path}" <<'EOF'
# Lumelo development / bring-up images allow direct root SSH debugging.
PermitRootLogin yes
PasswordAuthentication yes
EOF

    if [ -n "${SSH_AUTHORIZED_KEYS_FILE}" ]; then
      install -d -m 0700 "${ROOTFS_DIR}/root/.ssh"
      install -m 0600 "${SSH_AUTHORIZED_KEYS_FILE}" \
        "${ROOTFS_DIR}/root/.ssh/authorized_keys"
    fi
  else
    rm -f "${ROOTFS_DIR}/etc/systemd/system/multi-user.target.wants/ssh.service"
    rm -f "${dev_sshd_config_path}"
    rm -rf "${ROOTFS_DIR}/root/.ssh"
  fi
fi

config_path="${ROOTFS_DIR}/etc/lumelo/config.toml"
if [ -f "${config_path}" ]; then
  sed -i "s/^ssh_enabled = .*/ssh_enabled = ${ssh_enabled_value}/" "${config_path}"
fi

rm -f "${ROOTFS_DIR}/etc/ssh/ssh_host_"*

if [ -n "${ROOT_PASSWORD}" ]; then
  if [ -x "${ROOTFS_DIR}/usr/sbin/chpasswd" ]; then
    printf 'root:%s\n' "${ROOT_PASSWORD}" | chroot "${ROOTFS_DIR}" /usr/sbin/chpasswd
    root_password_set=1
  else
    echo "ROOT_PASSWORD was provided, but chpasswd is not available in rootfs" >&2
    exit 1
  fi
fi

: > "${ROOTFS_DIR}/etc/machine-id"
mkdir -p "${ROOTFS_DIR}/var/lib/dbus"
ln -sf /etc/machine-id "${ROOTFS_DIR}/var/lib/dbus/machine-id"

rm -rf "${ROOTFS_DIR}/var/cache/apt/archives/"*.deb
rm -rf "${ROOTFS_DIR}/var/lib/apt/lists/"*

cat > "${ROOTFS_DIR}/etc/lumelo/image-build.txt" <<EOF
Lumelo-defined rootfs image profile: ${PROFILE}
Board support source: ${BOARD_SOURCE_IMAGE}
Built at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
SSH enabled in image: ${ENABLE_SSH}
SSH authorized_keys injected: $(if [ -n "${SSH_AUTHORIZED_KEYS_FILE}" ]; then printf '1'; else printf '0'; fi)
Root console password set: ${root_password_set}
EOF
~~~

## `base/rootfs/manifests/README.md`

- bytes: 92
- segment: 1/1

~~~md
# Rootfs Manifests

Place package manifests, file lists, and image composition inputs here.
~~~

## `base/rootfs/manifests/t4-bringup-packages.txt`

- bytes: 447
- segment: 1/1

~~~text
# Minimal package set for the first Lumelo-defined T4 bring-up rootfs.
# This profile favors a clean systemd/network stack and local diagnostics.

alsa-utils
bash
bluez
ca-certificates
curl
dbus
firmware-brcm80211
firmware-misc-nonfree
iw
iproute2
iputils-ping
kmod
libnss-resolve
libnss-systemd
netbase
passwd
procps
python3
python3-dbus
python3-gi
rfkill
systemd
systemd-resolved
systemd-sysv
systemd-timesyncd
udev
wireless-regdb
wpasupplicant
~~~

## `base/rootfs/overlay/etc/NetworkManager/NetworkManager.conf`

- bytes: 57
- segment: 1/1

~~~ini
[main]
plugins=ifupdown,keyfile

[ifupdown]
managed=true
~~~

## `base/rootfs/overlay/etc/NetworkManager/conf.d/12-managed-wifi.conf`

- bytes: 49
- segment: 1/1

~~~ini
[keyfile]
unmanaged-devices=wl*,except:type:wifi
~~~

## `base/rootfs/overlay/etc/NetworkManager/conf.d/99-unmanaged-wlan1.conf`

- bytes: 49
- segment: 1/1

~~~ini
[keyfile]
unmanaged-devices=interface-name:wlan1
~~~

## `base/rootfs/overlay/etc/NetworkManager/conf.d/disable-random-mac-during-wifi-scan.conf`

- bytes: 39
- segment: 1/1

~~~ini
[device]
wifi.scan-rand-mac-address=no
~~~

## `base/rootfs/overlay/etc/bluetooth/main.conf`

- bytes: 119
- segment: 1/1

~~~ini
[General]
Name = Lumelo T4
ControllerMode = dual
DiscoverableTimeout = 0
PairableTimeout = 0

[Policy]
AutoEnable=true
~~~

## `base/rootfs/overlay/etc/dbus-1/system.d/org.lumelo.provisioning.conf`

- bytes: 347
- segment: 1/1

~~~ini
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
  "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="root">
    <allow own="org.lumelo.provisioning"/>
  </policy>

  <policy context="default">
    <allow send_destination="org.lumelo.provisioning"/>
  </policy>
</busconfig>
~~~

## `base/rootfs/overlay/etc/lumelo/config.toml`

- bytes: 228
- segment: 1/1

~~~toml
# Writable runtime configuration.
# Development images can seed this from the default config on first boot.

mode = "local"
interface_mode = "ethernet"
dsd_output_policy = "strict_native"
ssh_enabled = false
ui_theme = "system"
~~~

## `base/rootfs/overlay/etc/lumelo/sessiond.env`

- bytes: 461
- segment: 1/1

~~~text
# Quiet Mode service boundaries.
#
# sessiond may freeze only the explicitly non-critical media work domain.
# Networking, control-plane access, and remote recovery services must stay
# reachable on a headless device.

SESSIOND_PROTECTED_SERVICES="controld.service systemd-networkd.service NetworkManager.service wpa_supplicant.service iwd.service dhcpcd.service avahi-daemon.service ssh.service sshd.service"
SESSIOND_FREEZABLE_SERVICES="media-indexd.service"
~~~

## `base/rootfs/overlay/etc/network/interfaces`

- bytes: 134
- segment: 1/1

~~~text
# interfaces(5) file used by ifup(8) and ifdown(8)
# Include files from /etc/network/interfaces.d:
source /etc/network/interfaces.d/*
~~~

## `base/rootfs/overlay/etc/systemd/network/20-wired-dhcp.network`

- bytes: 164
- segment: 1/1

~~~text
[Match]
Name=en* eth*

[Network]
DHCP=yes
LinkLocalAddressing=no
LLMNR=no
MulticastDNS=no

[DHCPv4]
RouteMetric=100
ClientIdentifier=mac

[IPv6AcceptRA]
UseDNS=yes
~~~

## `base/rootfs/overlay/etc/systemd/network/30-wireless-dhcp.network`

- bytes: 165
- segment: 1/1

~~~text
[Match]
Name=wlan* wl*

[Network]
DHCP=yes
LinkLocalAddressing=no
LLMNR=no
MulticastDNS=no

[DHCPv4]
RouteMetric=200
ClientIdentifier=mac

[IPv6AcceptRA]
UseDNS=yes
~~~

## `base/rootfs/overlay/etc/systemd/resolved.conf.d/lumelo.conf`

- bytes: 35
- segment: 1/1

~~~ini
[Resolve]
LLMNR=no
MulticastDNS=no
~~~

## `base/rootfs/overlay/etc/systemd/system/auth-recovery.service`

- bytes: 360
- segment: 1/1

~~~ini
[Unit]
Description=Reset admin authentication from physical recovery media
DefaultDependencies=no
After=local-fs.target
Before=controld.service local-mode.target bridge-mode.target
ConditionPathExists=/usr/libexec/lumelo/auth-recovery

[Service]
Type=oneshot
ExecStart=/usr/libexec/lumelo/auth-recovery

[Install]
WantedBy=local-mode.target bridge-mode.target
~~~

## `base/rootfs/overlay/etc/systemd/system/bluetooth.service.d/10-lumelo-rfkill-unblock.conf`

- bytes: 232
- segment: 1/1

~~~ini
[Unit]
After=systemd-rfkill.service systemd-rfkill.socket
Wants=systemd-rfkill.service

[Service]
ExecStartPre=/bin/sh -c 'if command -v rfkill >/dev/null 2>&1; then rfkill unblock bluetooth || true; rfkill unblock all || true; fi'
~~~

## `base/rootfs/overlay/etc/systemd/system/bluetooth.service.d/20-lumelo-uart-attach.conf`

- bytes: 96
- segment: 1/1

~~~ini
[Unit]
Requires=lumelo-bluetooth-uart-attach.service
After=lumelo-bluetooth-uart-attach.service
~~~

## `base/rootfs/overlay/etc/systemd/system/bridge-mode.target`

- bytes: 168
- segment: 1/1

~~~text
[Unit]
Description=Lumelo Bridge Mode Placeholder Target
Wants=auth-recovery.service
After=auth-recovery.service
AllowIsolate=yes

[Install]
WantedBy=multi-user.target
~~~

## `base/rootfs/overlay/etc/systemd/system/controld.service`

- bytes: 540
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Web Control Service
After=auth-recovery.service sessiond.service network.target
Requires=playbackd.service
ConditionPathExists=/usr/bin/controld

[Service]
Type=simple
Environment=LUMELO_RUNTIME_DIR=/run/lumelo
Environment=CONTROLD_LISTEN_ADDR=0.0.0.0:18080
ExecStart=/usr/bin/controld
Restart=on-failure
RestartSec=2
RuntimeDirectory=lumelo
RuntimeDirectoryMode=0755
RuntimeDirectoryPreserve=yes
StateDirectory=lumelo
StateDirectoryMode=0755
WorkingDirectory=/var/lib/lumelo

[Install]
WantedBy=local-mode.target
~~~

## `base/rootfs/overlay/etc/systemd/system/local-mode.target`

- bytes: 420
- segment: 1/1

~~~text
[Unit]
Description=Lumelo Local Mode Target
Documentation=file:/usr/share/lumelo/default_config.toml
Wants=auth-recovery.service playbackd.service sessiond.service media-indexd.service controld.service lumelo-media-reconcile.service
After=auth-recovery.service playbackd.service sessiond.service media-indexd.service controld.service lumelo-media-reconcile.service
AllowIsolate=yes

[Install]
WantedBy=multi-user.target
~~~

## `base/rootfs/overlay/etc/systemd/system/lumelo-bluetooth-provisioning.service`

- bytes: 296
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Bluetooth Provisioning Mode
After=bluetooth.service dbus.service
Requires=bluetooth.service
ConditionPathExists=/usr/bin/bluetoothctl

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/bin/lumelo-bluetooth-provisioning-mode

[Install]
WantedBy=multi-user.target
~~~

## `base/rootfs/overlay/etc/systemd/system/lumelo-bluetooth-uart-attach.service`

- bytes: 497
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Bluetooth UART Attach
After=local-fs.target systemd-modules-load.service
Wants=systemd-modules-load.service
Before=bluetooth.service lumelo-bluetooth-provisioning.service lumelo-wifi-provisiond.service
ConditionPathExists=/usr/bin/hciattach.rk
ConditionPathExists=/usr/libexec/lumelo/bluetooth-uart-attach
ConditionPathExists=/dev/ttyS0

[Service]
Type=oneshot
ExecStart=/usr/libexec/lumelo/bluetooth-uart-attach
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
~~~

## `base/rootfs/overlay/etc/systemd/system/lumelo-media-import@.service`

- bytes: 237
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Removable Media Import for /dev/%I
After=local-fs.target
ConditionPathExists=/usr/bin/lumelo-media-import

[Service]
Type=oneshot
ExecStart=/usr/bin/lumelo-media-import import-device /dev/%I
TimeoutStartSec=120
~~~

## `base/rootfs/overlay/etc/systemd/system/lumelo-media-reconcile.service`

- bytes: 217
- segment: 1/1

~~~ini
[Unit]
Description=Lumelo Removable Media Availability Reconcile
After=local-fs.target
ConditionPathExists=/usr/bin/lumelo-media-import

[Service]
Type=oneshot
ExecStart=/usr/bin/lumelo-media-import reconcile-volumes
~~~

