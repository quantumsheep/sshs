#!/bin/sh
set -e

cp /tmp/authorized_keys /home/testuser/.ssh/authorized_keys
chown testuser:testuser /home/testuser/.ssh/authorized_keys
chmod 600 /home/testuser/.ssh/authorized_keys

exec $@
