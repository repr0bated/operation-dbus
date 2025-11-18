#!/bin/bash
# OVSDB JSON-RPC helper script
# Usage: ovsdb-rpc.sh <json-request>

echo "$1" | socat - UNIX-CONNECT:/var/run/openvswitch/db.sock
