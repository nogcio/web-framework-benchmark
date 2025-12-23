import pkg from '../../package.json'

export const REPO_URL = pkg.repository.url.replace(/\.git$/, '')
