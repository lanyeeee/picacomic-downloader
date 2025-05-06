import { DownloadTaskEvent } from './bindings.ts'

export type CurrentTabName = 'search' | 'favorite' | 'downloaded' | 'chapter'

export type ProgressData = DownloadTaskEvent & { percentage: number; indicator: string }
