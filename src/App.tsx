import { Advanced } from './components/Advanced';
import { Alert } from './components/Alert';
import { Diagnostics } from './components/Diagnostics';
import { Disclosure } from './components/Disclosure';
import { Roster } from './components/Roster';
import { useLocalName } from './hooks/useLocalName';
import { useTransport } from './hooks/useTransport';

const App = () => {
	const me = useLocalName();
	const {
		port,
		setPort,
		peer,
		setPeer,
		stats,
		peers,
		error,
		start,
		stop,
		running
	} = useTransport();

	return (
		<main className='min-h-screen bg-slate-50 px-6 py-8 text-slate-900 dark:bg-slate-950 dark:text-slate-100'>
			<div className='mx-auto flex w-full max-w-lg flex-col gap-5'>
				<header className='flex items-start justify-between'>
					<div>
						<h1 className='text-xl font-semibold tracking-tight'>
							Intercom
						</h1>
						<p className='text-xs text-slate-500 dark:text-slate-400'>
							{me ? `You are ${me}` : ' '}
						</p>
					</div>
					<button
						type='button'
						onClick={running ? stop : start}
						className={`rounded-lg px-4 py-2 text-sm font-medium text-white transition ${
							running
								? 'bg-slate-700 hover:bg-slate-600'
								: 'bg-teal-600 hover:bg-teal-500'
						}`}
					>
						{running ? 'Stop' : 'Start'}
					</button>
				</header>

				{error && <Alert message={error} />}

				<Roster
					peers={peers}
					running={running}
					selected={peer}
					onSelect={setPeer}
				/>

				<Disclosure label='Advanced'>
					<Advanced
						port={port}
						peer={peer}
						onPort={setPort}
						onPeer={setPeer}
						disabled={running}
					/>
				</Disclosure>

				<Disclosure label='Diagnostics'>
					<Diagnostics stats={stats} running={running} />
				</Disclosure>
			</div>
		</main>
	);
};

export default App;
