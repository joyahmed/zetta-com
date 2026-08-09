import { useState } from 'react';

/// A plain show/hide, optionally driven from outside.
///
/// Advanced settings, diagnostics and the key list all belong behind one: they
/// are worth keeping — the counters are what found a firewall problem nothing
/// else would have named — but they are not what the screen is for. The
/// controlled mode exists so a global shortcut can open the right one.
export const Disclosure = ({
	label,
	children,
	open,
	onOpenChange
}: DisclosureProps) => {
	const [internal, setInternal] = useState(false);
	const isOpen = open ?? internal;

	const toggle = () => {
		const next = !isOpen;
		setInternal(next);
		onOpenChange?.(next);
	};

	return (
		<section className='flex flex-col gap-2'>
			<button
				type='button'
				onClick={toggle}
				className='self-start text-xs text-slate-500 underline-offset-2 hover:underline dark:text-slate-400'
			>
				{isOpen ? `Hide ${label.toLowerCase()}` : label}
			</button>
			{isOpen && children}
		</section>
	);
};
