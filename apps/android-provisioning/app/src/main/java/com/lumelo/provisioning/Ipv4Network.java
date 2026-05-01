package com.lumelo.provisioning;

import android.net.ConnectivityManager;
import android.net.LinkAddress;
import android.net.LinkProperties;
import android.net.Network;

import java.net.Inet4Address;
import java.net.InetAddress;
import java.util.List;

final class Ipv4Network {
    private Ipv4Network() {
    }

    static AddressInfo currentAddressInfo(ConnectivityManager manager) {
        if (manager == null) {
            return null;
        }
        Network activeNetwork = manager.getActiveNetwork();
        if (activeNetwork == null) {
            return null;
        }
        LinkProperties properties = manager.getLinkProperties(activeNetwork);
        if (properties == null) {
            return null;
        }
        List<LinkAddress> addresses = properties.getLinkAddresses();
        if (addresses == null) {
            return null;
        }
        for (LinkAddress address : addresses) {
            InetAddress inetAddress = address.getAddress();
            if (inetAddress instanceof Inet4Address && !inetAddress.isLoopbackAddress()) {
                int prefixLength = address.getPrefixLength();
                if (prefixLength < 0 || prefixLength > 32) {
                    prefixLength = 32;
                }
                return new AddressInfo(inetAddress.getHostAddress(), prefixLength);
            }
        }
        return null;
    }

    static boolean isPrivateIpv4(String ip) {
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

    static boolean sameSubnet(AddressInfo addressInfo, String targetIp) {
        if (addressInfo == null) {
            return false;
        }
        Integer left = parseIpv4(addressInfo.address);
        Integer right = parseIpv4(targetIp);
        if (left == null || right == null) {
            return false;
        }
        int prefixLength = addressInfo.prefixLength;
        if (prefixLength <= 0) {
            return true;
        }
        if (prefixLength >= 32) {
            return left.intValue() == right.intValue();
        }
        int mask = (int) (0xffffffffL << (32 - prefixLength));
        return (left & mask) == (right & mask);
    }

    private static Integer parseIpv4(String ip) {
        String[] parts = ip.split("\\.");
        if (parts.length != 4) {
            return null;
        }
        int value = 0;
        for (String part : parts) {
            int octet;
            try {
                octet = Integer.parseInt(part);
            } catch (NumberFormatException exception) {
                return null;
            }
            if (octet < 0 || octet > 255) {
                return null;
            }
            value = (value << 8) | octet;
        }
        return value;
    }

    static final class AddressInfo {
        final String address;
        final int prefixLength;

        AddressInfo(String address, int prefixLength) {
            this.address = address;
            this.prefixLength = prefixLength;
        }
    }
}
