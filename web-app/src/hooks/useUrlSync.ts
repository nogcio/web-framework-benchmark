import { useEffect, useRef } from 'react'
import { useAppStore } from '../store/useAppStore'
import type { VisibleColumns } from '../types'

export function useUrlSync() {
  const isInitialized = useRef(false)
  
  const selectedRunId = useAppStore(s => s.selectedRunId)
  const selectedEnvironment = useAppStore(s => s.selectedEnvironment)
  const selectedTest = useAppStore(s => s.selectedTest)
  const visibleColumns = useAppStore(s => s.visibleColumns)
  
  const setSelectedRunId = useAppStore(s => s.setSelectedRunId)
  const setSelectedEnvironment = useAppStore(s => s.setSelectedEnvironment)
  const setSelectedTest = useAppStore(s => s.setSelectedTest)
  const setVisibleColumns = useAppStore(s => s.setVisibleColumns)

  // Initialize from URL on mount
  useEffect(() => {
    if (isInitialized.current) return
    
    const params = new URLSearchParams(window.location.search)
    
    const env = params.get('env')
    if (env) {
      setSelectedEnvironment(env)
    }
    
    const test = params.get('test')
    if (test) {
      setSelectedTest(test)
    }

    const runId = params.get('run')
    if (runId) {
      setSelectedRunId(Number(runId))
    }
    
    const cols = params.get('cols')
    if (cols) {
      const colKeys = cols.split(',')
      const newCols: VisibleColumns = {
        rank: false,
        framework: false,
        rps: false,
        memory: false,
        memoryBar: false,
        tps: false,
        tpsBar: false,
        errors: false,
        tags: false,
      }
      colKeys.forEach(key => {
        if (key in newCols) {
          newCols[key as keyof VisibleColumns] = true
        }
      })
      setVisibleColumns(newCols)
    }
    
    isInitialized.current = true
  }, [setSelectedRunId, setSelectedEnvironment, setSelectedTest, setVisibleColumns])

  // Update URL when state changes
  useEffect(() => {
    if (!isInitialized.current) return

    const params = new URLSearchParams()
    
    if (selectedRunId) {
      params.set('run', selectedRunId.toString())
    }
    
    if (selectedEnvironment) {
      params.set('env', selectedEnvironment)
    }
    
    if (selectedTest) {
      params.set('test', selectedTest)
    }
    
    // Only serialize visible columns
    const activeCols = Object.entries(visibleColumns)
      .filter(([_, visible]) => visible)
      .map(([key]) => key)
      .join(',')
      
    if (activeCols) {
      params.set('cols', activeCols)
    }

    const newUrl = `${window.location.pathname}?${params.toString()}`
    window.history.replaceState(null, '', newUrl)
    
  }, [selectedRunId, selectedEnvironment, selectedTest, visibleColumns])
}
