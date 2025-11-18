# Privacy Router OpenFlow Obfuscation Levels

The privacy router integrates with the OpenFlow plugin's three obfuscation levels to provide progressive traffic hiding from ISP DPI and traffic analysis.

## Obfuscation Levels

### Level 0: No Obfuscation (NOT RECOMMENDED)
```json
{
  "privacy_router": {
    "openflow": {
      "enabled": true,
      "enable_security_flows": false,
      "obfuscation_level": 0
    }
  }
}
```

**Flows**: 0 (no protection)

**Use Case**: Testing only - exposes traffic patterns to ISP

---

### Level 1: Basic Security (DEFAULT)
```json
{
  "privacy_router": {
    "openflow": {
      "enabled": true,
      "enable_security_flows": true,
      "obfuscation_level": 1
    }
  }
}
```

**Flows**: 11+ security flows (cookies: 0xDEAD0001-0xDEAD0016)

**Protection**:
- Drop invalid TCP flags (NULL/Xmas/FIN scans)
- Drop IP fragmentation attacks
- Rate limit ARP/ICMP to prevent storms
- Drop invalid source IPs (0.0.0.0, multicast)
- Drop broadcast source MAC
- Connection tracking (stateful firewall)
- Egress filtering:
  * Block TTL <=1 (prevent traceroute leakage)
  * Block reserved IPs (240.0.0.0/4)
  * Rate limit port scans (SYN packets)
  * Rate limit DNS, NTP, SNMP, LDAP, SSDP

**Recommended for**: Basic privacy router, prevents ISP ban

---

### Level 2: Pattern Hiding (RECOMMENDED)
```json
{
  "privacy_router": {
    "openflow": {
      "enabled": true,
      "enable_security_flows": true,
      "obfuscation_level": 2
    }
  }
}
```

**Flows**: 11 (Level 1) + 3 (Level 2) = 14 flows total
- Level 1: 0xDEAD#### cookies
- Level 2: 0xCAFE#### cookies

**Protection** (Level 1 + Level 2):
- **TTL Normalization**: Rewrite all outbound TTL to 64 (prevent OS fingerprinting)
- **Packet Size Markers**: Mark packets for padding (hide payload size patterns)
- **Timing Randomization**: Vary flow idle timeouts (prevent timing analysis)

**Recommended for**: Standard privacy router, hides traffic patterns from DPI

---

### Level 3: Advanced Obfuscation (MAXIMUM STEALTH)
```json
{
  "privacy_router": {
    "openflow": {
      "enabled": true,
      "enable_security_flows": true,
      "obfuscation_level": 3,
      "controller_endpoint": "tcp:127.0.0.1:6633"
    }
  }
}
```

**Flows**: 11 (Level 1) + 3 (Level 2) + 4 (Level 3) = 18 flows total
- Level 1: 0xDEAD#### cookies
- Level 2: 0xCAFE#### cookies
- Level 3: 0xBEEF#### cookies

**Protection** (Level 1 + Level 2 + Level 3):
- **Protocol Mimicry**: WireGuard UDP:51820 → HTTPS TCP:443 (disguise VPN as web traffic)
- **Decoy Traffic**: Inject random noise packets (prevent traffic analysis)
- **HTTPS Shaping**: Mimic browser HTTPS timing patterns
- **Fragment Randomization**: Hide true packet sizes via fragmentation

**Note**: Requires OpenFlow controller for full functionality (decoy injection, true rate limiting)

**Recommended for**: Maximum stealth, evade sophisticated DPI and traffic analysis

---

## Privacy Router Integration

The privacy router automatically applies obfuscation flows to the privacy tunnel bridge (vmbr0):

```json
{
  "privacy_router": {
    "bridge_name": "vmbr0",
    "openflow": {
      "enabled": true,
      "enable_security_flows": true,
      "obfuscation_level": 2,
      "privacy_flows": [
        {
          "priority": 100,
          "match_fields": {"in_port": "internal_100"},
          "actions": ["output:warp0"],
          "description": "WireGuard gateway → WARP tunnel"
        },
        {
          "priority": 100,
          "match_fields": {"in_port": "warp0"},
          "actions": ["output:internal_101"],
          "description": "WARP → XRay client"
        }
      ]
    }
  }
}
```

## Flow Priority Order

OpenFlow flows are applied in priority order:

1. **32000+**: Security flows (Level 1) - Highest priority
2. **30000-31000**: Connection tracking, egress filtering (Level 1)
3. **29000**: Pattern hiding flows (Level 2)
4. **28000**: Advanced obfuscation flows (Level 3)
5. **100-200**: Privacy routing flows (user-defined)
6. **10**: Default fallback

## Implementation

The OpenFlow plugin automatically generates flows based on `obfuscation_level`:

```rust
// Level 1: Basic security (always enabled if enable_security_flows=true)
if obfuscation_level >= 1 {
    let security_flows = generate_security_flows(&bridge_name);
    // 11+ flows with 0xDEAD#### cookies
}

// Level 2: Pattern hiding
if obfuscation_level >= 2 {
    let pattern_flows = generate_pattern_hiding_flows(&bridge_name);
    // 3 flows with 0xCAFE#### cookies
}

// Level 3: Advanced obfuscation
if obfuscation_level >= 3 {
    let advanced_flows = generate_advanced_obfuscation_flows(&bridge_name);
    // 4 flows with 0xBEEF#### cookies
}
```

## Verification

Check applied flows:

```bash
# View all flows on privacy bridge
ovs-ofctl dump-flows vmbr0

# Filter by cookie to see obfuscation level
ovs-ofctl dump-flows vmbr0 | grep "cookie=0xDEAD"  # Level 1
ovs-ofctl dump-flows vmbr0 | grep "cookie=0xCAFE"  # Level 2
ovs-ofctl dump-flows vmbr0 | grep "cookie=0xBEEF"  # Level 3
```

## Performance Impact

- **Level 1**: Minimal impact (~1-2% CPU)
- **Level 2**: Low impact (~2-5% CPU)
- **Level 3**: Moderate impact (~5-10% CPU, requires controller)

## Recommendations

- **Home/Personal**: Level 2 (pattern hiding)
- **High-Risk Environment**: Level 3 (advanced obfuscation)
- **Testing/Development**: Level 0 or 1

