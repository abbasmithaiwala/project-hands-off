import { ImagePlus } from 'lucide-react'
import { useCallback, useEffect, useRef, useState, type DragEvent } from 'react'
import { ThemeToggle } from '@/app/ThemeToggle'
import { describeError } from '@/api/errors'
import { useUploadImage } from '@/api/hooks'
import { Button } from '@/design/Button'
import { Logo } from '@/design/Logo'
import { Skeleton } from '@/design/Skeleton'
import { toast } from '@/design/toastStore'
import { isSupportedImage } from '@/lib/files'
import { useEditorStore } from '@/state/editorStore'
import styles from './LandingScreen.module.css'
import { BlobBackdrop } from './BlobBackdrop'
import { RecentProjects } from './RecentProjects'

const SAMPLE_URL = '/sample/couch.jpg'

export function LandingScreen() {
  const openImage = useEditorStore((state) => state.openImage)
  const upload = useUploadImage()
  const [dragging, setDragging] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  const submit = useCallback(
    (file: File | undefined) => {
      if (!file || upload.isPending) return
      if (!isSupportedImage(file)) {
        toast.error('Please choose a JPEG, PNG or WebP image.')
        return
      }
      upload.mutate(file, {
        onSuccess: openImage,
        onError: (error) => toast.error(describeError(error)),
      })
    },
    [openImage, upload],
  )

  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => {
      const file = [...(event.clipboardData?.files ?? [])].find(isSupportedImage)
      if (file) submit(file)
    }
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [submit])

  const trySample = async () => {
    try {
      const response = await fetch(SAMPLE_URL)
      if (!response.ok) throw new Error('sample unavailable')
      submit(new File([await response.blob()], 'sample-couch.jpg', { type: 'image/jpeg' }))
    } catch {
      toast.error('The sample photo could not be loaded. Try uploading your own.')
    }
  }

  const onDrop = (event: DragEvent) => {
    event.preventDefault()
    setDragging(false)
    submit(event.dataTransfer.files[0])
  }

  return (
    <main className={styles.screen}>
      <BlobBackdrop />
      <header className={styles.top}>
        <span className={styles.brand}>
          <Logo size={40} />
          <span className={styles.name}>Hands-Off</span>
        </span>
        <span className={styles.tagline}>Open-source photo editing, by hand</span>
        <ThemeToggle />
      </header>
      <div className={styles.hero}>
        <h1 className={styles.title}>Edit by dragging, not typing.</h1>
        <p className={styles.subtitle}>
          Drop a photo, pick an object, and reshape it with a handle. No prompts needed.
        </p>
      </div>
      <div
        className={styles.zone}
        data-dragging={dragging}
        data-testid="drop-zone"
        onDragOver={(event) => {
          event.preventDefault()
          setDragging(true)
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={onDrop}
      >
        <div className={styles.mark} aria-hidden="true">
          <ImagePlus size={28} />
        </div>
        {upload.isPending ? (
          <div className={styles.progress} role="status">
            <Skeleton width={200} height={10} />
            Uploading…
          </div>
        ) : (
          <>
            <Button variant="primary" size="large" onClick={() => inputRef.current?.click()}>
              Choose an image
            </Button>
            <p className={styles.hint}>or drop it here, or paste from your clipboard</p>
            <Button variant="ghost" onClick={() => void trySample()}>
              Try a sample photo
            </Button>
            <p className={styles.credit}>
              Sample by BORNTHISWAYMEDIA,{' '}
              <a
                href="https://commons.wikimedia.org/wiki/File:ORANGE_SOFA.jpg"
                target="_blank"
                rel="noreferrer"
              >
                CC BY-SA 4.0
              </a>
            </p>
          </>
        )}

        <input
          ref={inputRef}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          className="visually-hidden"
          aria-hidden="true"
          tabIndex={-1}
          data-testid="file-input"
          onChange={(event) => {
            submit(event.target.files?.[0])
            event.target.value = ''
          }}
        />
      </div>
      <RecentProjects />
    </main>
  )
}
