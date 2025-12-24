import { useEffect, useState } from 'react'
import { Star, Github } from 'lucide-react'
import { Button } from './ui/button'
import pkg from '../../package.json'

export function GitHubStars() {
  const [stars, setStars] = useState<number | null>(null)

  // Parse repo URL to get owner/repo
  // Format: https://github.com/owner/repo.git or git+https://github.com/owner/repo.git
  const repoUrl = pkg.repository.url.replace(/\.git$/, '').replace(/^git\+/, '')
  const repoPath = repoUrl.split('github.com/')[1]

  useEffect(() => {
    if (!repoPath) return

    fetch(`https://api.github.com/repos/${repoPath}`)
      .then(res => res.json())
      .then(data => setStars(data.stargazers_count))
      .catch(console.error)
  }, [repoPath])

  if (!repoPath) return null

  return (
    <Button variant="outline" size="sm" className="gap-2" asChild>
      <a href={repoUrl} target="_blank" rel="noreferrer">
        <Github className="h-4 w-4" />
        <span className="font-semibold hidden sm:inline">Star</span>
        <span className="flex items-center gap-1 text-muted-foreground ml-1">
            <Star className="h-3.5 w-3.5" />
            {stars !== null ? stars.toLocaleString() : '...'}
        </span>
      </a>
    </Button>
  )
}
