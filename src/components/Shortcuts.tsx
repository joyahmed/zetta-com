/// Every key and what it does — including the ones that failed.
///
/// A list that only showed intentions would be worse than none, because it
/// reads as a promise. A global shortcut can lose a registration race to any
/// other application on the machine, and when it does the key simply does
/// nothing, which is otherwise indistinguishable from the app being broken.
///
/// The nine talk keys and the nine message keys are collapsed into one row
/// each. Listed individually they were eighteen of twenty-six rows, all saying
/// almost the same thing, and the four keys that actually differ were lost in
/// the middle of them.
/// The two verbs are not written the same way in `keys.rs` — "Talk to PC 1" but
/// "Message PC 1" — so the whole prefix is captured rather than the verb plus an
/// assumed " to ". Matching on the verb alone silently collapsed one range and
/// left the other as nine rows.
const RANGE = /^(Talk to|Message) PC (\d+)$/;

type Row = {
	label: string;
	keys: string;
	/// How many of the keys behind this row failed to register. Kept as a count
	/// rather than a flag: "two of nine" is a different problem from "all nine",
	/// and collapsing the rows must not collapse that distinction away.
	failed: number;
	total: number;
};

const collapse = (shortcuts: ShortcutInfo[]): Row[] => {
	const out: Row[] = [];
	const done = new Set<string>();

	for (const s of shortcuts) {
		const kind = s.label.match(RANGE)?.[1];
		if (!kind) {
			out.push({
				label: s.label,
				keys: s.keys,
				failed: s.registered ? 0 : 1,
				total: 1
			});
			continue;
		}
		// The whole range is emitted where its first key appeared, so the list
		// keeps the order it was registered in.
		if (done.has(kind)) continue;
		done.add(kind);

		const group = shortcuts.filter(x => x.label.match(RANGE)?.[1] === kind);
		const nums = group.map(x => x.label.match(RANGE)![2]);
		const span = `${nums[0]}…${nums[nums.length - 1]}`;
		out.push({
			label: `${kind} PC ${nums[0]}–${nums[nums.length - 1]}`,
			// The trailing digit is what varies; everything before it is the
			// modifier the whole range shares.
			keys: group[0].keys.replace(/\d+$/, span),
			failed: group.filter(x => !x.registered).length,
			total: group.length
		});
	}

	return out;
};

export const Shortcuts = ({ shortcuts }: ShortcutsProps) => {
	if (shortcuts.length === 0) return null;
	const rows = collapse(shortcuts);
	const failed = shortcuts.filter(s => !s.registered).length;

	return (
		<div className='flex flex-col gap-2'>
			{failed > 0 && (
				<p className='text-xs text-danger'>
					{failed} shortcut{failed === 1 ? '' : 's'} could not be
					registered — another application already owns{' '}
					{failed === 1 ? 'it' : 'them'}.
				</p>
			)}

			<div className='overflow-hidden rounded-lg border border-line bg-sunken'>
				{rows.map(r => (
					<div
						key={`${r.label}-${r.keys}`}
						className='flex items-center gap-3 border-b border-line-soft px-3 py-1.5 text-sm last:border-0'
					>
						<span
							className={`flex-1 truncate ${
								r.failed === 0
									? ''
									: r.failed === r.total
										? 'text-faint line-through'
										: 'text-muted'
							}`}
						>
							{r.label}
							{/* A partly-registered range must say so. Struck
							    through it would claim none of it works; plain it
							    would claim all of it does. */}
							{r.failed > 0 && r.failed < r.total && (
								<span className='text-danger'>
									{' '}
									— {r.failed} of {r.total} unavailable
								</span>
							)}
						</span>
						<kbd
							className={`shrink-0 rounded border px-1.5 py-0.5 font-mono text-xs ${
								r.failed === 0
									? 'border-line text-muted'
									: 'border-danger text-danger'
							}`}
						>
							{r.keys}
						</kbd>
					</div>
				))}
			</div>

			{/* The roster has no limit; the number row does. Said plainly here
			    rather than left to be discovered by pressing a key for the
			    tenth PC and having nothing happen — which is exactly the silent
			    failure the rest of this list exists to prevent. */}
			<p className='text-xs text-muted'>
				The numbers follow the order PCs appear in the list, so 1–9 are
				the first nine. You can add as many PCs as you like — there are
				only nine number keys. For the rest, pick one and hold F8.
			</p>
		</div>
	);
};
