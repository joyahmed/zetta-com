import { invoke } from '@tauri-apps/api/core';
import { useEffect, useRef, useState } from 'react';

type NetStats = {
	port: number;
	peer: string;
	tx: number;
	rx: number;
	bad: number;
	lost: number;
	lastSeq: number;
};

type Saved = { port: string; peer: string };

const STORE_KEY = 'zc.transport';

const loadSaved = (): Saved => {
	try {
		const raw = localStorage.getItem(STORE_KEY);
		if (raw) return JSON.parse(raw) as Saved;
	} catch {
		// Unreadable storage is not worth failing over — fall through to defaults.
	}
	return { port: '9001', peer: '127.0.0.1:9002' };
};

type FieldProps = {
	label: string;
	value: string;
	onChange: (v: string) => void;
	disabled?: boolean;
	placeholder?: string;
	className?: string;
};

const Field = ({
	label,
	value,
	onChange,
	disabled,
	placeholder,
	className = ''
}: FieldProps) => (
	<label className={`flex flex-col gap-1 text-left ${className}`}>
		<span className='text-xs font-medium tracking-wide text-slate-500 uppercase dark:text-slate-400'>
			{label}
		</span>
		<input
			value={value}
			onChange={e => onChange(e.currentTarget.value)}
			disabled={disabled}
			placeholder={placeholder}
			className='rounded-lg border border-slate-300 bg-white px-3 py-2 font-mono text-sm text-slate-900 outline-none transition placeholder:text-slate-400 focus:border-teal-500 focus:ring-2 focus:ring-teal-500/30 disabled:cursor-not-allowed disabled:opacity-55 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100'
		/>
	</label>
);

type StatProps = { label: string; value?: number; warn?: boolean };

const Stat = ({ label, value, warn }: StatProps) => (
	<div className='flex flex-col items-center rounded-xl border border-slate-200 bg-white px-3 py-3 dark:border-slate-800 dark:bg-slate-900'>
		{/* Loss and rejects have to be visible without being read: at a glance the
        panel is either all neutral or it is not. */}
		<span
			className={`text-2xl font-semibold tabular-nums ${
				warn
					? 'text-rose-600 dark:text-rose-400'
					: 'text-slate-900 dark:text-slate-100'
			}`}
		>
			{value ?? '—'}
		</span>
		<span className='mt-0.5 text-[0.7rem] tracking-wide text-slate-500 uppercase dark:text-slate-400'>
			{label}
		</span>
	</div>
);

const App = () => {
	const saved = useRef(loadSaved()).current;
	const [port, setPort] = useState(saved.port);
	const [peer, setPeer] = useState(saved.peer);
	const [stats, setStats] = useState<NetStats | null>(null);
	const [error, setError] = useState('');

	useEffect(() => {
		localStorage.setItem(STORE_KEY, JSON.stringify({ port, peer }));
	}, [port, peer]);

	// One poll answers both questions: net_stats returns null exactly when the
	// transport is stopped, so there is no separate "is it running" call that
	// could drift out of agreement with reality.
	useEffect(() => {
		let alive = true;
		const id = setInterval(async () => {
			try {
				const s = await invoke<NetStats | null>('net_stats');
				if (alive) setStats(s);
			} catch (e) {
				if (alive) setError(String(e));
			}
		}, 250);
		// Without this, every re-render stacks another interval and the poll rate
		// quietly climbs until the webview is doing nothing else.
		return () => {
			alive = false;
			clearInterval(id);
		};
	}, []);

	const start = async () => {
		setError('');
		const p = Number(port);
		// Checked here so a typo reads as a sentence rather than as an IPC
		// serialisation failure about u16.
		if (!Number.isInteger(p) || p < 1 || p > 65535) {
			setError('Port must be a whole number between 1 and 65535.');
			return;
		}
		try {
			await invoke('net_start', { port: p, peer });
		} catch (e) {
			setError(String(e));
		}
	};

	const stop = async () => {
		setError('');
		try {
			await invoke('net_stop');
			setStats(null);
		} catch (e) {
			setError(String(e));
		}
	};

	const running = stats !== null;

	return (
		<main className='min-h-screen bg-slate-50 px-6 py-10 text-slate-900 dark:bg-slate-950 dark:text-slate-100'>
			<div className='mx-auto flex w-full max-w-xl flex-col gap-6'>
				<header className='flex items-baseline justify-between'>
					<h1 className='text-xl font-semibold tracking-tight'>
						Transport
					</h1>
					<span className='flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400'>
						<span
							className={`size-2 rounded-full ${
								running
									? 'bg-teal-500'
									: 'bg-slate-400 dark:bg-slate-600'
							}`}
						/>
						{running ? 'running' : 'stopped'}
					</span>
				</header>

				<form
					className='flex items-end gap-3'
					onSubmit={e => {
						e.preventDefault();
						if (running) stop();
						else start();
					}}
				>
					<Field
						label='Port'
						value={port}
						onChange={setPort}
						disabled={running}
						className='w-28'
					/>
					<Field
						label='Peer'
						value={peer}
						onChange={setPeer}
						disabled={running}
						placeholder='127.0.0.1:9002'
						className='flex-1'
					/>
					<button
						type='submit'
						className={`rounded-lg px-4 py-2 text-sm font-medium text-white transition ${
							running
								? 'bg-slate-700 hover:bg-slate-600'
								: 'bg-teal-600 hover:bg-teal-500'
						}`}
					>
						{running ? 'Stop' : 'Start'}
					</button>
				</form>

				{error && (
					<p className='rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:border-rose-900 dark:bg-rose-950 dark:text-rose-300'>
						{error}
					</p>
				)}

				<div className='grid grid-cols-5 gap-2'>
					<Stat label='sent' value={stats?.tx} />
					<Stat label='received' value={stats?.rx} />
					<Stat
						label='lost'
						value={stats?.lost}
						warn={!!stats?.lost}
					/>
					<Stat
						label='rejected'
						value={stats?.bad}
						warn={!!stats?.bad}
					/>
					<Stat label='last seq' value={stats?.lastSeq} />
				</div>

				<p className='text-xs text-slate-500 dark:text-slate-400'>
					{running
						? `bound 0.0.0.0:${stats?.port} → ${stats?.peer}`
						: 'not bound'}
				</p>
			</div>
		</main>
	);
};

export default App;
