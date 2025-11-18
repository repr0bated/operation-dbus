#!/bin/bash
# Direct OVSDB operations - no AI needed!

case "$1" in
    "delete-ovsbr0")
        echo '{"jsonrpc":"2.0","id":"1","method":"tools/call","params":{"name":"json_rpc_call","arguments":{"method":"transact","params":["Open_vSwitch",{"op":"delete","table":"Bridge","where":[["name","==","ovsbr0"]]}]}}}' | sudo /git/operation-dbus/target/release/dbus-mcp
        ;;
    "delete-ovsbr1")
        echo '{"jsonrpc":"2.0","id":"2","method":"tools/call","params":{"name":"json_rpc_call","arguments":{"method":"transact","params":["Open_vSwitch",{"op":"delete","table":"Bridge","where":[["name","==","ovsbr1"]]}]}}}' | sudo /git/operation-dbus/target/release/dbus-mcp
        ;;
    "list-bridges")
        sudo /git/operation-dbus/ovsdb-rpc.sh '{"method":"transact","params":["Open_vSwitch",{"op":"select","table":"Bridge","where":[]}],"id":0}' | jq '.result[0].rows'
        ;;
    *)
        echo "Usage: $0 {delete-ovsbr0|delete-ovsbr1|list-bridges}"
        exit 1
        ;;
esac
