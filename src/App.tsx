import { useState } from 'react';
import { AddPc } from './components/AddPc';
import { Alert } from './components/Alert';
import { Connection } from './components/Connection';
import { Devices } from './components/Devices';
import { Diagnostics } from './components/Diagnostics';
import { Messages } from './components/Messages';
import { Modal } from './components/Modal';
import { Nav } from './components/Nav';
import { Pcs } from './components/Pcs';
import { Presets } from './components/Presets';
import { Roster } from './components/Roster';
import { Shortcuts } from './components/Shortcuts';
import { TalkBar } from './components/TalkBar';
import { Targets } from './components/Targets';
import { useLocalName } from './hooks/useLocalName';
import { useManualPeers } from './hooks/useManualPeers';
import { useMessages } from './hooks/useMessages';
import { usePtt } from './hooks/usePtt';
import { useShortcuts } from './hooks/useShortcuts';
import { useDevices } from './hooks/useDevices';
import { useTarget } from './hooks/useTarget';
import { useTransport } from './hooks/useTransport';

const App = () => {
	const me = useLocalName();
	const held = usePtt();
	const {
		port,
		setPort,
		stats,
		peers,
		error,
		setError,
		start,
		stop,
		running
	} = useTransport();
	const { messages, send } = useMessages(running);
	const { manual, presets, add, remove, edit, rename, reorder } =
		useManualPeers(setError);
	const { target, setTarget } = useTarget(setError);
	const devices = useDevices(setError);
	const {
		shortcuts,
		setShortcut,
		showShortcuts,
		setShowShortcuts,
		showAddPc,
		setShowAddPc
	} = useShortcuts();
	const [showSettings, setShowSettings] = useState(false);
	const [showDiagnostics, setShowDiagnostics] = useState(false);
	const [showPeople, setShowPeople] = useState(false);

	// Named for the talk bar and the message box, so it always says who is
	// about to be addressed rather than leaving it to be remembered.
	const to =
		target === null
			? 'everyone'
			: (peers.find(p => p.addr === target)?.name ?? target);

	return (
		// h-screen rather than min-h-screen: the log sizes itself against the
		// window, and a container that can grow past it has nothing to size
		// against.
		<div className='flex h-screen flex-col overflow-hidden bg-canvas text-ink'>
			<Nav
				{...{
					me,
					running,
					onToggle: running ? stop : start,
					onAddPc: () => setShowAddPc(true),
					onShortcuts: () => setShowShortcuts(true),
					onSettings: () => setShowSettings(true),
					onDiagnostics: () => setShowDiagnostics(true)
				}}
			/>

			{/* The main screen is the two things this app is: who you are
			    talking to, and what has been said. Everything else is behind
			    the bar.
			    min-h-0 is load-bearing on a flex column whose child scrolls —
			    without it the log refuses to shrink below its content and
			    pushes the input off the bottom instead of scrolling. */}
			<main className='mx-auto flex min-h-0 w-full max-w-md flex-1 flex-col gap-3 p-4'>
				{error && <Alert {...{ message: error }} />}

				{running && <TalkBar {...{ held, key_: 'F8', to }} />}

				<Targets
					{...{
						peers,
						running,
						target,
						onTarget: setTarget,
						onSeeAll: () => setShowPeople(true)
					}}
				/>

				<Messages
					{...{
						messages,
						onSend: send,
						disabled: !running,
						to,
						quick: (
							<Presets
								{...{ presets, onSend: send, disabled: !running }}
							/>
						)
					}}
				/>
			</main>

			{/* The strip is for aiming; this is for reading. Addresses, live
			    state and the slot numbers the shortcuts use are all here, with
			    room to show them. */}
			<Modal
				{...{
					title: 'All PCs',
					open: showPeople,
					onClose: () => setShowPeople(false)
				}}
			>
				<Roster
					{...{
						peers,
						running,
						target,
						onTarget: (addr: string | null) => {
							setTarget(addr);
							setShowPeople(false);
						}
					}}
				/>
			</Modal>

			<Modal
				{...{
					title: 'Shortcuts',
					open: showShortcuts,
					onClose: () => setShowShortcuts(false)
				}}
			>
				<Shortcuts {...{ shortcuts, onSet: setShortcut }} />
			</Modal>

			{/* Adding only. Managing what you have added is a different job
			    done at a different time, and it lives in Settings. */}
			<Modal
				{...{
					title: 'Add a PC',
					open: showAddPc,
					onClose: () => setShowAddPc(false)
				}}
			>
				<div className='flex flex-col gap-2'>
					<AddPc {...{ onAdd: add, onName: rename }} />
					<p className='text-xs text-muted'>
						Only needed where discovery is blocked, or for a PC on
						another subnet. Rename or remove one in Settings.
					</p>
				</div>
			</Modal>

			{/* Two things: the port, and the PCs. Both change how the app
			    behaves and both change rarely, which is what a setting is. */}
			<Modal
				{...{
					title: 'Settings',
					open: showSettings,
					onClose: () => setShowSettings(false)
				}}
			>
				<div className='flex flex-col gap-5'>
					<Connection {...{ port, onPort: setPort, disabled: running }} />

					<div className='flex flex-col gap-2'>
						<h3 className='text-xs font-medium tracking-wide text-muted uppercase'>
							Audio
						</h3>
						<Devices
							{...{
								inputs: devices.inputs,
								outputs: devices.outputs,
								input: devices.input,
								output: devices.output,
								onChoose: devices.choose,
								onRefresh: devices.refresh
							}}
						/>
					</div>

					<div className='flex flex-col gap-2'>
						<h3 className='text-xs font-medium tracking-wide text-muted uppercase'>
							PCs
						</h3>
						<Pcs
							{...{
								peers,
								manual,
								onRename: rename,
								onEdit: edit,
								onRemove: remove,
								onReorder: reorder
							}}
						/>
					</div>
				</div>
			</Modal>

			<Modal
				{...{
					title: 'Diagnostics',
					open: showDiagnostics,
					onClose: () => setShowDiagnostics(false)
				}}
			>
				<Diagnostics {...{ stats, running }} />
			</Modal>
		</div>
	);
};

export default App;
