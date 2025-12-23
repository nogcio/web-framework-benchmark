import { Github, Mail } from 'lucide-react'
import { useEffect, useState } from 'react'
import pkg from '../../package.json'

export default function Footer() {
  const [backendVersion, setBackendVersion] = useState<string | null>(null)

  useEffect(() => {
    fetch('/api/version')
      .then(res => res.json())
      .then(data => setBackendVersion(data.version))
      .catch(() => setBackendVersion('unknown'))
  }, [])

  return (
    <footer className="mt-auto py-2 border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container mx-auto px-6 flex flex-col md:flex-row items-center justify-between gap-2 text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
        <div className="flex items-center gap-2">
          <span>Open Source Project</span>
          <span>•</span>
          <span>MIT License</span>
          <span>•</span>
          <span>2025 Web Framework Benchmarks</span>
        </div>
        
        <div className="flex items-center gap-4">
          <span>Client: v{pkg.version}</span>
          {backendVersion && <span>Backend: v{backendVersion}</span>}
          <a 
            href="mailto:getansum@nogc.io" 
            className="flex items-center gap-1.5 hover:text-foreground transition-colors"
          >
            <Mail className="h-3 w-3" />
            <span>getansum@nogc.io</span>
          </a>
          
          <a 
            href="https://github.com/nogcio/web-framework-benchmark" 
            target="_blank" 
            rel="noopener noreferrer" 
            className="flex items-center gap-1.5 hover:text-foreground transition-colors"
          >
            <Github className="h-3 w-3" />
            <span>GitHub</span>
          </a>
        </div>
      </div>
    </footer>
  )
}
