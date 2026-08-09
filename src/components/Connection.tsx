import { Field } from './Field';

/// The port this machine binds.
///
/// A setting rather than part of adding a PC, and not something you touch while
/// the transport is running, which is why it disables rather than disappears.
///
/// There used to be a "Fallback address" beside it — a single typed address for
/// networks that filter mDNS. It was the same thing as a manually added PC (the
/// session merged it straight into that list) but it lived in localStorage
/// instead of the config, so it appeared in nobody's PC list and could not be
/// removed. Whatever was in it is migrated into the PCs list on first launch.
export const Connection = ({ port, onPort, disabled }: ConnectionProps) => (
	<div className='flex flex-col gap-3'>
		<Field
			{...{
				label: 'Port',
				value: port,
				onChange: onPort,
				disabled,
				className: 'w-24'
			}}
		/>
		<p className='text-xs text-slate-500 dark:text-slate-400'>
			{disabled
				? 'Stop the transport to change this.'
				: 'Everyone must use the same port. Add a PC that discovery cannot reach from the PCs list.'}
		</p>
	</div>
);
