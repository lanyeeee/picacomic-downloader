import { DownloadTaskEvent } from './bindings.ts'

export type CurrentTabName = 'search' | 'favorite' | 'rank' | 'downloaded' | 'chapter'

export type ProgressData = Extract<DownloadTaskEvent, { event: 'Create' }>['data'] & {
  percentage: number
  indicator: string
}
