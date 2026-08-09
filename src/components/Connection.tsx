import { Field } from './Field';

/// Port and a fallback address.
///
/// Both are settings rather than part of adding a PC: the port is this
/// machine's, and the address is the escape hatch for a network where discovery
/// is filtered. Neither is something you touch while the transport is running,
/// which is why they disable rather than disappear.
export const Connection = ({
	port,
	peer,
	onPort,
	onPeer,
	disabled
}: ConnectionProps) => (
	<div className='flex flex-col gap-3'>
		<div className='flex items-end gap-3'>
			<Field
				{...{
					label: 'Port',
					value: port,
					onChange: onPort,
					disabled,
					className: 'w-24'
				}}
			/>
			<Field
				{...{
					label: 'Fallback address',
					value: peer,
					onChange: onPeer,
					disabled,
					placeholder: '192.168.0.142:9001',
					className: 'flex-1'
				}}
			/>
		</div>
		<p className='text-xs text-slate-500 dark:text-slate-400'>
			{disabled
				? 'Stop the transport to change these.'
				: 'Everyone must use the same port. The address is only needed where discovery is blocked.'}
		</p>
	</div>
);
