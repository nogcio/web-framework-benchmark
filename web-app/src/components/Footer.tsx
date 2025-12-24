import { Github, Mail } from 'lucide-react'
import { useEffect, useState } from 'react'
import { REPO_URL } from '../lib/constants'

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
      <div className="w-full px-4 flex items-center justify-end gap-3 md:gap-6 text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
        {backendVersion && <span>v{backendVersion}</span>}
        
        <a 
          href="mailto:getansum@nogc.io" 
          className="flex items-center gap-1.5 hover:text-foreground transition-colors"
        >
          <Mail className="h-3 w-3" />
          <span className="hidden md:inline">getansum@nogc.io</span>
        </a>
        
        <a 
          href={REPO_URL}
          target="_blank" 
          rel="noopener noreferrer" 
          className="flex items-center gap-1.5 hover:text-foreground transition-colors"
        >
          <Github className="h-3 w-3" />
          <span className="hidden md:inline">GitHub</span>
        </a>
      </div>
    </footer>
  )
}
