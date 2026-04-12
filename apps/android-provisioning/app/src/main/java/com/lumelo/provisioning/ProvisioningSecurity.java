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
