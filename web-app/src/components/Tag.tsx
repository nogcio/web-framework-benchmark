import { getColorForString } from '../lib/utils'

interface TagProps {
  k: string
  v?: string
}

export function Tag({ k, v }: TagProps) {
  const color = getColorForString(k)
  return (
    <span
      className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-sm border bg-background/50"
      style={{
        borderColor: color,
        color: color
      }}
    >
      <span className="font-medium text-xs">{k}</span>
      {v ? <span className="text-xs opacity-80">{v}</span> : null}
    </span>
  )
}
