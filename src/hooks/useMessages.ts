import { invoke } from '@tauri-apps/api/core';
import {
	isPermissionGranted,
	requestPermission,
	sendNotification
} from '@tauri-apps/plugin-notification';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useRef, useState } from 'react';
import { DEMO, DEMO_MESSAGES } from '../demo';
import { chime } from '../utils/chime';

const POLL_MS = 600;

/// Text is its own hook for the same reason it is its own command: in v1 voice
/// and text went out on one keypress, and when only one of them arrived there
/// was no way to tell which half had failed.
export const useMessages = (running: boolean) => {
	const [messages, setMessages] = useState<Message[]>(
		DEMO ? DEMO_MESSAGES : []
	);
	// Highest id already seen. Notifying on anything above it means a restart
	// never replays the whole log at you as a burst of toasts.
	const seen = useRef(0);
	const allowed = useRef(false);

	useEffect(() => {
		(async () => {
			allowed.current =
				(await isPermissionGranted()) ||
				(await requestPermission()) === 'granted';
		})().catch(() => {});
	}, []);

	useEffect(() => {
		if (DEMO) return;
		if (!running) {
			setMessages([]);
			return;
		}
		let alive = true;
		const id = setInterval(async () => {
			try {
				const m = await invoke<Message[]>('messages');
				if (!alive) return;
				setMessages(m);

				// Only what arrived, and only when the window is not being
				// looked at. This app lives in the tray; a notification for a
				// message already on screen is noise.
				//
				// The window is asked, not the document. `document.hidden` was
				// the obvious test and it is wrong here: a window hidden to the
				// tray still reports its document as visible in WebView2, and a
				// window sitting behind another application never sets it at
				// all — so the condition was almost never true and a
				// notification almost never fired.
				//
				// Checked only when something has actually arrived, so this
				// costs two IPC calls per message rather than two per poll.
				const fresh = m.filter(x => x.id > seen.current && !x.mine);
				if (allowed.current && fresh.length > 0) {
					const w = getCurrentWindow();
					const attended = await Promise.all([w.isVisible(), w.isFocused()])
						.then(([visible, focused]) => visible && focused)
						.catch(() => false);
					if (!attended) {
						// Once, however many arrived together. A chime per
						// message turns three at the same moment into a noise
						// nobody can count.
						chime();
						for (const msg of fresh) {
							sendNotification({
								title: msg.from || 'Intercom',
								body: msg.text
							});
						}
					}
				}
				seen.current = Math.max(seen.current, ...m.map(x => x.id), 0);
			} catch {
				// The transport reports its own failures; a missed poll here
				// would only duplicate them.
			}
		}, POLL_MS);
		return () => {
			alive = false;
			clearInterval(id);
		};
	}, [running]);

	const send = async (text: string) => {
		await invoke('send_text', { text });
	};

	return { messages, send };
};
