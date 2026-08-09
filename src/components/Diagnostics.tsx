import { Stat } from './Stat';

export const Diagnostics = ({ stats, running }: DiagnosticsProps) => (
	<>
		<div className='grid grid-cols-5 gap-2'>
			<Stat label='sent' value={stats?.tx} />
			<Stat label='received' value={stats?.rx} />
			<Stat label='lost' value={stats?.lost} warn={!!stats?.lost} />
			<Stat label='rejected' value={stats?.bad} warn={!!stats?.bad} />
			<Stat label='last seq' value={stats?.lastSeq} />
		</div>
		<p className='font-mono text-xs text-slate-500 dark:text-slate-400'>
			{running
				? `bound 0.0.0.0:${stats?.port} → ${stats?.peer}`
				: 'not bound'}
		</p>
	</>
);
