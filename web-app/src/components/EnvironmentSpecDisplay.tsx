import { type Environment } from '../types'

interface Props {
  environment: Environment
}

export function EnvironmentSpecDisplay({ environment }: Props) {
  return (
    <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-xs">
      {(environment.spec || '').split('\n').filter(line => line.trim()).map((line, i) => {
        const colonIndex = line.indexOf(':')
        // Treat as key-value if there is a colon, but not if it looks like a header (e.g. ends with colon)
        if (colonIndex > 0 && colonIndex < line.length - 1) {
          const key = line.slice(0, colonIndex).trim()
          const value = line.slice(colonIndex + 1).trim()
          return (
            <div key={i} className="contents">
              <div className="text-muted-foreground font-medium whitespace-nowrap">{key}:</div>
              <div className="text-foreground">{value}</div>
            </div>
          )
        }
        return (
          <div key={i} className="col-span-2 font-semibold text-foreground pt-2 first:pt-0 border-b border-border/40 pb-1 mb-1 last:border-0 last:mb-0 last:pb-0">
            {line}
          </div>
        )
      })}
      {!environment.spec && (
        <div className="col-span-2 text-muted-foreground italic">No specification available</div>
      )}
    </div>
  )
}
