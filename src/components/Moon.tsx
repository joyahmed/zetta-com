/// The do-not-disturb mark: a crescent, the glyph phones already taught
/// everyone. Used for this machine's switch and beside anyone who is busy, so
/// the one shape means the one thing everywhere it appears.
export const Moon = ({
	size = 12,
	filled = false
}: {
	size?: number;
	filled?: boolean;
}) => (
	<svg
		width={size}
		height={size}
		viewBox='0 0 24 24'
		fill={filled ? 'currentColor' : 'none'}
		stroke='currentColor'
		strokeWidth='2'
		strokeLinecap='round'
		strokeLinejoin='round'
		aria-hidden='true'
		className='shrink-0'
	>
		<path d='M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z' />
	</svg>
);
