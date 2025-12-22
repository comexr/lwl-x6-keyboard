install:
	mkdir -p ~/.rusty-kb
	mkdir -p ~/.local/share/applications
	mkdir -p ~/.config/systemd/user
	cargo build --release
	sudo cp target/release/keyboard-controller /bin/
	cp icon.png ~/.rusty-kb/
	cp setcolor.sh ~/.rusty-kb/
	cp rusty-kb.desktop ~/.local/share/applications/
	sudo cp rusty-kb.service ~/.config/systemd/user/rusty-kb.service
	systemctl --user daemon-reload
	systemctl --user enable rusty-kb.service
	systemctl --user start rusty-kb.service
	chmod +x ~/.rusty-kb/setcolor.sh
	sudo chmod +x /bin/keyboard-controller
uninstall:
	sudo rm -f /bin/keyboard-controller
	rm -f ~/.rusty-kb/icon.png
	rm -f ~/.rusty-kb/setcolor.sh
	rm -f ~/.local/share/applications/rusty-kb.desktop.desktop
	systemctl --user disable rusty-kb.service
	rm -f ~/.config/systemd/user/rusty-kb.service
	systemctl --user daemon-reload

clean:s
	cargo clean
