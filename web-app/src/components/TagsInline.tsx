import { useState } from 'react'
import { Tag } from './Tag'
import { getSortedTags } from '../lib/utils'

export function TagsInline({ tags }: { tags: Record<string, string> | undefined }) {
  const entries = getSortedTags(tags)
  const limit = 3
  const [open, setOpen] = useState(false)

  if (entries.length === 0) {
    return <span className="text-sm text-muted-foreground">—</span>
  }

  const visible = open ? entries : entries.slice(0, limit)
  const hiddenCount = Math.max(0, entries.length - limit)

  return (
    <div className="flex items-center gap-2">
      {visible.map(([k, v]) => (
        <Tag key={`${k}-${v}`} k={k} v={v} />
      ))}

      {hiddenCount > 0 && (
        <button
          type="button"
          className="text-xs text-muted-foreground ml-1 px-2 py-0.5 rounded hover:bg-muted/20"
          onClick={() => setOpen(!open)}
        >
          {open ? 'show less' : `+${hiddenCount}`}
        </button>
      )}
    </div>
  )
}
