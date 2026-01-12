import { useState, useEffect } from 'react'
import { useAppStore, type AppState } from '../store/useAppStore'
import { Home, Server, Laptop, Cloud, X, Info } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from './ui/card'
import { Button } from './ui/button'
import { EnvironmentSpecDisplay } from './EnvironmentSpecDisplay'

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
      <div className="fixed bottom-14 right-4 z-50 hidden md:block">
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
    <div className="fixed bottom-14 right-4 z-50 w-80 max-w-[calc(100vw-2rem)] hidden md:block">
      <Card className="shadow-lg border-border/60 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 gap-0">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2 p-4 border-b border-border/40">
          <CardTitle className="text-sm font-medium flex items-center gap-2 mb-0">
            <Icon className="h-4 w-4" />
            {selectedEnvironment.displayName}
          </CardTitle>
          <Button variant="ghost" size="icon" className="h-6 w-6 -mr-2" onClick={() => setIsOpen(false)}>
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>
        <CardContent className="p-4">
          <EnvironmentSpecDisplay environment={selectedEnvironment} />
        </CardContent>
      </Card>
    </div>
  )
}
