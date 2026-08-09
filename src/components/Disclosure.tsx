import { useState } from 'react';

/// A plain show/hide. Advanced settings and diagnostics both belong behind one:
/// they are worth keeping — the counters are what found a firewall problem that
/// nothing else would have named — but they are not what the screen is for.
export const Disclosure = ({ label, children }: DisclosureProps) => {
	const [open, setOpen] = useState(false);

	return (
		<section className='flex flex-col gap-2'>
			<button
				type='button'
				onClick={() => setOpen(v => !v)}
				className='self-start text-xs text-slate-500 underline-offset-2 hover:underline dark:text-slate-400'
			>
				{open ? `Hide ${label.toLowerCase()}` : label}
			</button>
			{open && children}
		</section>
	);
};
