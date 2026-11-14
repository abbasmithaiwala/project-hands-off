import { X } from 'lucide-react'
import { useEffect } from 'react'
import { cn } from '@/lib/cn'
import styles from './Toaster.module.css'
import { useToastStore, type Toast } from './toastStore'

const AUTO_DISMISS_MS = 6000

function ToastItem({ toast }: { toast: Toast }) {
  const dismiss = useToastStore((state) => state.dismiss)
  useEffect(() => {
    const timer = window.setTimeout(() => dismiss(toast.id), AUTO_DISMISS_MS)
    return () => window.clearTimeout(timer)
  }, [dismiss, toast.id])

  return (
    <div className={cn(styles.toast, toast.tone === 'error' && styles.error)}>
      <span className={styles.message}>{toast.message}</span>
      <button className={styles.close} aria-label="Dismiss" onClick={() => dismiss(toast.id)}>
        <X size={14} />
      </button>
    </div>
  )
}

export function Toaster() {
  const toasts = useToastStore((state) => state.toasts)
  return (
    <div className={styles.region} role="status" aria-live="polite">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} />
      ))}
    </div>
  )
}
