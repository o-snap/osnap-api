#!/bin/bash
if (( $EUID != 0 )); then
    echo "Please run as root"
    exit
fi

if [[ -e dbCommands ]]; then
	cp dbCommands /tmp/
else
	echo "Cannot find dbCommands file. Please place it in $(pwd)."
	exit
fi
ogdir=$(pwd)
cd /tmp
echo "This script was tested on Rocky Linux 9.1 but should work on any of the RedHat family"
inst=false
if [[ $(psql >/dev/null 2>&1) -eq 127 ]]; then
	echo "can't find postgresql!"
	inst=true
fi
if [[ -e /etc/redhat-release ]]; then
	echo "OS version OK"
	pkg="dnf"
elif [[ -e /etc/debain-release ]]; then
	echo "This is a debian system, not guaranteed to work."
	pkg="apt"
else
	echo "I don't know what OS I'm running on, auto-install not available."
	inst=false
fi

if [[ inst ]]; then
	$pkg install postgresql postgresql-server -y
	postgresql-setup --initdb
	systemctl enable --now postgresql
fi
echo "Switching to postgres default user"
su postgres -c 'createdb osnap && psql osnap -f /tmp/dbCommands'
if [[ "$?" -eq 0 ]]; then
	echo "successfully initialized database."
	cd $ogdir
else
	echo "failed :("
fi
exit
