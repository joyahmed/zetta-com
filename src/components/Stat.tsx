export const Stat = ({ label, value, warn }: StatProps) => (
	<div className='flex flex-col items-center rounded-lg border border-slate-200 bg-white px-2 py-2 dark:border-slate-800 dark:bg-slate-900'>
		{/* Loss and rejects have to be visible without being read: at a glance
		    the panel is either all neutral or it is not. */}
		<span
			className={`text-lg font-semibold tabular-nums ${
				warn
					? 'text-rose-600 dark:text-rose-400'
					: 'text-slate-900 dark:text-slate-100'
			}`}
		>
			{value ?? '—'}
		</span>
		<span className='text-[0.65rem] tracking-wide text-slate-500 uppercase dark:text-slate-400'>
			{label}
		</span>
	</div>
);
