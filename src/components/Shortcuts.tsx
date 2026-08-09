/// Every key and what it does — including the ones that failed.
///
/// A list that only showed intentions would be worse than none, because it
/// reads as a promise. A global shortcut can lose a registration race to any
/// other application on the machine, and when it does the key simply does
/// nothing, which is otherwise indistinguishable from the app being broken.
export const Shortcuts = ({ shortcuts }: ShortcutsProps) => {
	if (shortcuts.length === 0) return null;
	const failed = shortcuts.filter(s => !s.registered).length;

	return (
		<div className='flex flex-col gap-2'>
			{failed > 0 && (
				<p className='text-xs text-rose-600 dark:text-rose-400'>
					{failed} shortcut{failed === 1 ? '' : 's'} could not be
					registered — another application already owns{' '}
					{failed === 1 ? 'it' : 'them'}.
				</p>
			)}

			<div className='overflow-hidden rounded-lg border border-slate-200 dark:border-slate-800'>
				{shortcuts.map(s => (
					<div
						key={`${s.label}-${s.keys}`}
						className='flex items-center gap-3 border-b border-slate-100 px-3 py-1.5 text-sm last:border-0 dark:border-slate-800/60'
					>
						<span
							className={`flex-1 truncate ${
								s.registered
									? ''
									: 'text-slate-400 line-through dark:text-slate-600'
							}`}
						>
							{s.label}
						</span>
						<kbd
							className={`shrink-0 rounded border px-1.5 py-0.5 font-mono text-xs ${
								s.registered
									? 'border-slate-300 text-slate-600 dark:border-slate-700 dark:text-slate-300'
									: 'border-rose-300 text-rose-600 dark:border-rose-900 dark:text-rose-400'
							}`}
						>
							{s.keys}
						</kbd>
					</div>
				))}
			</div>
		</div>
	);
};
