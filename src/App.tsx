import { useState } from 'react';
import { Advanced } from './components/Advanced';
import { Alert } from './components/Alert';
import { Diagnostics } from './components/Diagnostics';
import { Messages } from './components/Messages';
import { Modal } from './components/Modal';
import { Nav } from './components/Nav';
import { PcNames } from './components/PcNames';
import { Presets } from './components/Presets';
import { Roster } from './components/Roster';
import { Shortcuts } from './components/Shortcuts';
import { TalkBar } from './components/TalkBar';
import { useLocalName } from './hooks/useLocalName';
import { useManualPeers } from './hooks/useManualPeers';
import { useMessages } from './hooks/useMessages';
import { usePtt } from './hooks/usePtt';
import { useShortcuts } from './hooks/useShortcuts';
import { useTarget } from './hooks/useTarget';
import { useTransport } from './hooks/useTransport';

const App = () => {
	const me = useLocalName();
	const held = usePtt();
	const {
		port,
		setPort,
		peer,
		setPeer,
		stats,
		peers,
		error,
		setError,
		start,
		stop,
		running
	} = useTransport();
	const { messages, send } = useMessages(running);
	const { manual, presets, add, remove, rename } = useManualPeers(setError);
	const { target, setTarget } = useTarget(setError);
	const {
		shortcuts,
		showShortcuts,
		setShowShortcuts,
		showAddPc,
		setShowAddPc
	} = useShortcuts();
	const [showSettings, setShowSettings] = useState(false);

	// Named for the talk bar and the message box, so it always says who is
	// about to be addressed rather than leaving it to be remembered.
	const to =
		target === null
			? 'everyone'
			: (peers.find(p => p.addr === target)?.name ?? target);

	return (
		<div className='flex min-h-screen flex-col bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100'>
			<Nav
				{...{
					me,
					running,
					onToggle: running ? stop : start,
					onAddPc: () => setShowAddPc(true),
					onShortcuts: () => setShowShortcuts(true),
					onSettings: () => setShowSettings(true)
				}}
			/>

			{/* The main screen is the two things this app is: who you are
			    talking to, and what has been said. Everything else is behind
			    the bar. */}
			<main className='flex flex-1 flex-col gap-4 p-4'>
				{error && <Alert {...{ message: error }} />}

				{running && <TalkBar {...{ held, key_: 'F8', to }} />}

				<Roster
					{...{
						peers,
						running,
						target,
						onTarget: setTarget
					}}
				/>

				<Presets {...{ presets, onSend: send, disabled: !running }} />

				<Messages
					{...{
						messages,
						onSend: send,
						disabled: !running,
						to
					}}
				/>
			</main>

			<Modal
				{...{
					title: 'Shortcuts',
					open: showShortcuts,
					onClose: () => setShowShortcuts(false)
				}}
			>
				<Shortcuts {...{ shortcuts }} />
			</Modal>

			<Modal
				{...{
					title: 'PCs',
					open: showAddPc,
					onClose: () => setShowAddPc(false)
				}}
			>
				<Advanced
					{...{
						port,
						peer,
						onPort: setPort,
						onPeer: setPeer,
						disabled: running,
						manual,
						onAdd: add,
						onRemove: remove
					}}
				/>

				<div className='mt-5 flex flex-col gap-2'>
					<h3 className='text-xs font-medium tracking-wide text-slate-500 uppercase dark:text-slate-400'>
						Names
					</h3>
					<PcNames {...{ peers, onRename: rename }} />
				</div>
			</Modal>

			<Modal
				{...{
					title: 'Settings',
					open: showSettings,
					onClose: () => setShowSettings(false)
				}}
			>
				<Diagnostics {...{ stats, running }} />
			</Modal>
		</div>
	);
};

export default App;
