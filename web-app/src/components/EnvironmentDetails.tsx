import { useState, useEffect } from 'react'
import { useAppStore, type AppState } from '../store/useAppStore'
import { Home, Server, Laptop, Cloud, X, Info } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from './ui/card'
import { Button } from './ui/button'

const iconMap: Record<string, React.ElementType> = {
  home: Home,
  server: Server,
  laptop: Laptop,
  cloud: Cloud,
}

export default function EnvironmentDetails() {
  const environments = useAppStore((s: AppState) => s.environments)
  const selectedEnvironmentName = useAppStore((s: AppState) => s.selectedEnvironment)
  const [isOpen, setIsOpen] = useState(true)

  const selectedEnvironment = environments.find(e => e.name === selectedEnvironmentName)

  // Re-open when environment changes
  useEffect(() => {
    if (selectedEnvironmentName) {
      setTimeout(() => setIsOpen(true), 0)
    }
  }, [selectedEnvironmentName])

  if (!selectedEnvironment) return null

  const Icon = iconMap[selectedEnvironment.icon] || Server

  if (!isOpen) {
    return (
      <div className="fixed bottom-14 right-4 z-50">
        <Button
          variant="outline"
          size="icon"
          className="rounded-full shadow-md bg-background"
          onClick={() => setIsOpen(true)}
        >
          <Info className="h-5 w-5" />
        </Button>
      </div>
    )
  }

  return (
    <div className="fixed bottom-14 right-4 z-50 w-80 max-w-[calc(100vw-2rem)]">
      <Card className="shadow-lg border-border/60 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2 p-4 border-b border-border/40">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Icon className="h-4 w-4" />
            {selectedEnvironment.displayName}
          </CardTitle>
          <Button variant="ghost" size="icon" className="h-6 w-6 -mr-2" onClick={() => setIsOpen(false)}>
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>
        <CardContent className="p-4">
          <div className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-xs">
            {(selectedEnvironment.spec || '').split('\n').filter(line => line.trim()).map((line, i) => {
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
            {!selectedEnvironment.spec && (
              <div className="col-span-2 text-muted-foreground italic">No specification available</div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
