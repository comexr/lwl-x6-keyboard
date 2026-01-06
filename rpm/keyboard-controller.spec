%define __spec_install_post %{nil}
%define __os_install_post %{_dbpath}/brp-compress
%define debug_package %{nil}

Name: lwl-x6-keyboard
Summary: A GTK4-based keyboard controller application
Version: @@VERSION@@
Release: @@RELEASE@@%{?dist}
License: MIT
Group: Applications/System
Source0: %{name}-%{version}.tar.gz

BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root

%description
%{summary}

%prep
%setup -q

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a * %{buildroot}

%clean
rm -rf %{buildroot}

%post
/usr/sbin/groupadd -r rustykb >/dev/null 2>&1 || true
add_user_to_group() {
    user="$1"
    [ -n "$user" ] || return 0
    /usr/bin/id "$user" >/dev/null 2>&1 || return 0
    /usr/sbin/usermod -a -G rustykb "$user" >/dev/null 2>&1 || true
}

USER_TO_ADD=${SUDO_USER:-$(/usr/bin/logname 2>/dev/null || true)}
add_user_to_group "$USER_TO_ADD"

# Also add all regular (uid >= 1000) login-capable users for unattended installs
while IFS=: read -r name _ uid _ _ _ shell; do
    case "$shell" in
        *nologin|false|"") continue ;;
    esac
    if [ "$uid" -ge 1000 ]; then
        add_user_to_group "$name"
    fi
done </etc/passwd

if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload >/dev/null 2>&1 || true
    udevadm trigger --subsystem-match=leds >/dev/null 2>&1 || true
fi

if command -v systemctl >/dev/null 2>&1; then
    # Enable the user service for all users so colors are restored after login
    systemctl --global enable rusty-kb.service >/dev/null 2>&1 || true
fi

%files
%defattr(-,root,root,-)
%{_bindir}/lwl-x6-keyboard
%{_datadir}/applications/rusty-kb.desktop
%{_datadir}/icons/hicolor/256x256/apps/rusty-kb.png
/usr/lib/rusty-kb/setcolor.sh
/usr/lib/systemd/user/rusty-kb.service
/etc/udev/rules.d/99-rusty-kb.rules
/usr/lib/rusty-kb/colors.txt
