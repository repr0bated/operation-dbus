#!/bin/sh
set -a
if [ -f /etc/op-dbus/environment ]; then
  . /etc/op-dbus/environment
fi
set +a

exec /usr/local/bin/op-dbus
