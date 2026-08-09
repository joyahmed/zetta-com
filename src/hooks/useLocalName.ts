import { invoke } from '@tauri-apps/api/core';
import { useEffect, useState } from 'react';

/// This machine's name, as everyone else's roster will show it. Read once —
/// it cannot change while the app is running.
export const useLocalName = () => {
	const [name, setName] = useState('');

	useEffect(() => {
		invoke<string>('local_name')
			.then(setName)
			.catch(() => {
				// Falls back to showing nothing, which is better than an error
				// about a label.
			});
	}, []);

	return name;
};
