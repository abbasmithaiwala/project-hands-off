import { describe, expect, it } from 'vitest'
import type { ImageRecord } from '@/api/client'
import { downloadName, editLog, projectName, timeAgo, versionTitle } from './versions'

const original = { id: 'a', name: 'Living room', created_at: '2026-01-01T00:00:00Z' } as ImageRecord
const edited = {
  id: 'b',
  name: 'Living room',
  created_at: '2026-01-01T01:00:00Z',
  edit: { summary: 'Shortened the couch', changes: [] },
} as unknown as ImageRecord

describe('versionTitle', () => {
  it('describes the edit, or calls the first version the original', () => {
    expect(versionTitle(original)).toBe('Original')
    expect(versionTitle(edited)).toBe('Shortened the couch')
  })
})

describe('projectName', () => {
  it('falls back to Untitled', () => {
    expect(projectName({ name: null })).toBe('Untitled')
    expect(projectName(original)).toBe('Living room')
  })
})

describe('downloadName', () => {
  it('makes a portable file name', () => {
    expect(downloadName('Living room / 2', 'png')).toBe('living-room-2.png')
    expect(downloadName('Café', 'json')).toBe('cafe.json')
  })

  it('never produces an empty base name', () => {
    expect(downloadName('///', 'png')).toBe('hands-off.png')
  })
})

describe('editLog', () => {
  it('lists every version in order with its edit', () => {
    const log = JSON.parse(editLog([original, edited]))
    expect(log.name).toBe('Living room')
    expect(log.versions.map((v: { version: number }) => v.version)).toEqual([0, 1])
    expect(log.versions[0].edit).toBeNull()
    expect(log.versions[1].edit.summary).toBe('Shortened the couch')
  })
})

describe('timeAgo', () => {
  const now = Date.parse('2026-01-02T00:00:00Z')

  it('uses the largest sensible unit', () => {
    expect(timeAgo('2026-01-01T21:00:00Z', now)).toBe('3 hours ago')
    expect(timeAgo('2025-12-31T00:00:00Z', now)).toBe('2 days ago')
    expect(timeAgo('2026-01-01T23:58:00Z', now)).toBe('2 minutes ago')
  })

  it('says now for moments ago', () => {
    expect(timeAgo('2026-01-01T23:59:59Z', now)).toBe('now')
  })
})
