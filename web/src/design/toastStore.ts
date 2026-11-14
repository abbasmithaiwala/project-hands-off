import { create } from 'zustand'

export type ToastTone = 'info' | 'error'

export interface Toast {
  id: number
  message: string
  tone: ToastTone
}

interface ToastState {
  toasts: Toast[]
  push: (message: string, tone?: ToastTone) => void
  dismiss: (id: number) => void
}

let nextId = 1
const MAX_TOASTS = 3

export const useToastStore = create<ToastState>()((set) => ({
  toasts: [],
  push: (message, tone = 'info') =>
    set((state) => ({
      toasts: [...state.toasts, { id: nextId++, message, tone }].slice(-MAX_TOASTS),
    })),
  dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
}))

export const toast = {
  info: (message: string) => useToastStore.getState().push(message, 'info'),
  error: (message: string) => useToastStore.getState().push(message, 'error'),
}
