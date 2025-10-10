<script setup lang="ts">
import { SelectionArea, SelectionEvent } from '@viselect/vue'
import { nextTick, ref, watch, watchEffect, computed } from 'vue'
import { ChapterInfo, commands, DownloadTaskState } from '../bindings.ts'
import { useStore } from '../store.ts'

const store = useStore()

const dropdownX = ref<number>(0)
const dropdownY = ref<number>(0)
const showDropdown = ref<boolean>(false)
const dropdownOptions = [
  { label: '勾选', key: 'check' },
  { label: '取消勾选', key: 'uncheck' },
  { label: '全选', key: 'check all' },
  { label: '取消全选', key: 'uncheck all' },
]
const checkedIds = ref<string[]>([])
const selectedIds = ref<Set<string>>(new Set())
const selectionAreaRef = ref<InstanceType<typeof SelectionArea>>()

type State = DownloadTaskState | 'Idle'
const chapterInfos = computed<(ChapterInfo & { state: State })[]>(() => {
  const pickedComic = store.pickedComic

  if (pickedComic === undefined) {
    return []
  }

  return pickedComic.chapterInfos.map((chapterInfo) => {
    const progressData = store.progresses.get(chapterInfo.chapterId)
    if (progressData === undefined) {
      return {
        ...chapterInfo,
        state: 'Idle',
      }
    }
    return {
      ...chapterInfo,
      state: progressData.state,
    }
  })
})

watch(
  () => store.pickedComic,
  () => {
    checkedIds.value = []
    selectedIds.value.clear()
    selectionAreaRef.value?.selection?.clearSelection()
  },
)

watchEffect(() => {
  if (store.pickedComic === undefined) {
    return
  }
  // 只保留未下载的章节
  // TODO: 改用set效率更高
  const notDownloadedChapterIds = chapterInfos.value
    .filter((c) => c.isDownloaded !== true && !isDownloading(c.state))
    .map((c) => c.chapterId)
  checkedIds.value = checkedIds.value.filter((id) => notDownloadedChapterIds.includes(id))
})

function extractIds(elements: Element[]): string[] {
  return elements
    .map((element) => element.getAttribute('data-key'))
    .filter(Boolean)
    .filter((id) => id !== null)
    .filter((id) => {
      const chapter = store.pickedComic?.chapterInfos.find((chapter) => chapter.chapterId === id)
      if (chapter === undefined) {
        return false
      }
      return chapter.isDownloaded !== true
    })
}

function unselectAll({ event, selection }: SelectionEvent) {
  if (!event?.ctrlKey && !event?.metaKey) {
    selection.clearSelection()
    selectedIds.value.clear()
  }
}

function updateSelectedIds({
  store: {
    changed: { added, removed },
  },
}: SelectionEvent) {
  extractIds(added).forEach((id) => selectedIds.value.add(id))
  extractIds(removed).forEach((id) => selectedIds.value.delete(id))
}

function onDropdownSelect(key: 'check' | 'uncheck' | 'check all' | 'uncheck all') {
  showDropdown.value = false
  if (key === 'check') {
    // 只有未勾选的才会被勾选
    ;[...selectedIds.value].filter((id) => !checkedIds.value.includes(id)).forEach((id) => checkedIds.value.push(id))
  } else if (key === 'uncheck') {
    checkedIds.value = checkedIds.value.filter((id) => !selectedIds.value.has(id))
  } else if (key === 'check all') {
    // 只有未锁定的才会被勾选
    store.pickedComic?.chapterInfos
      .filter((chapter) => chapter.isDownloaded !== true && !checkedIds.value.includes(chapter.chapterId))
      .forEach((chapter) => checkedIds.value.push(chapter.chapterId))
  } else if (key === 'uncheck all') {
    checkedIds.value.length = 0
  }
}

async function onContextMenu(e: MouseEvent) {
  showDropdown.value = false
  await nextTick()
  showDropdown.value = true
  dropdownX.value = e.clientX
  dropdownY.value = e.clientY
}

async function downloadChapters() {
  if (store.pickedComic === undefined) {
    return
  }
  // 下载勾选的章节
  const chapterIdsToDownload = store.pickedComic?.chapterInfos
    .filter((chapter) => chapter.isDownloaded !== true && checkedIds.value.includes(chapter.chapterId))
    .map((c) => c.chapterId)
  if (chapterIdsToDownload === undefined) {
    return
  }

  for (const chapterId of chapterIdsToDownload) {
    // 创建下载任务
    const result = await commands.createDownloadTask(store.pickedComic, chapterId)
    if (result.status === 'error') {
      console.error(result.error)
    }
  }
}

async function refreshChapters() {
  if (store.pickedComic === undefined) {
    return
  }
  const result = await commands.getComic(store.pickedComic.id.trim())
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  store.pickedComic = result.data
}

async function showComicDownloadDirInFileManager() {
  if (store.pickedComic === undefined || store.config === undefined) {
    return
  }

  const comicDownloadDir = store.pickedComic.comicDownloadDir
  if (comicDownloadDir === undefined || comicDownloadDir === null) {
    console.error('comicDownloadDir的值为undefined或null')
    return
  }

  const result = await commands.showPathInFileManager(comicDownloadDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}

function isDownloading(state: State) {
  return state === 'Pending' || state === 'Downloading' || state === 'Paused'
}
</script>

<template>
  <div class="h-full flex flex-col gap-2 box-border">
    <n-empty v-if="store.pickedComic === undefined" class="pt-2" description="请先进行漫画搜索" />
    <SelectionArea
      v-else
      ref="selectionAreaRef"
      class="selection-container flex flex-col flex-1 px-2 pt-0 overflow-auto"
      :options="{ selectables: '.selectable', features: { deselectOnBlur: true } }"
      @contextmenu="onContextMenu"
      @move="updateSelectedIds"
      @start="unselectAll">
      <div class="flex justify-between items-center pt-2">
        <div>左键拖动进行框选，右键打开菜单</div>
        <n-button size="small" @click="refreshChapters">刷新</n-button>
        <n-button size="small" type="primary" @click="downloadChapters">下载勾选章节</n-button>
      </div>
      <n-checkbox-group v-model:value="checkedIds" class="grid grid-cols-3 gap-1.5">
        <n-checkbox
          v-for="{ chapterId, chapterTitle, isDownloaded, state } in chapterInfos"
          :key="chapterId"
          :data-key="chapterId"
          class="selectable hover:bg-gray-200!"
          :value="chapterId"
          :label="chapterTitle"
          :disabled="isDownloaded === true || isDownloading(state)"
          :class="{
            selected: selectedIds.has(chapterId),
            downloaded: isDownloaded,
            downloading: !isDownloaded && isDownloading(state),
          }" />
      </n-checkbox-group>
    </SelectionArea>

    <div v-if="store.pickedComic !== undefined" class="flex p-2 pt-0">
      <img
        class="w-24 mr-4"
        :src="`${store.pickedComic.thumb.fileServer}/static/${store.pickedComic.thumb.path}`"
        alt=""
        referrerpolicy="no-referrer" />
      <div class="flex flex-col w-full justify-between">
        <div class="flex flex-col h-full">
          <span class="font-bold text-xl line-clamp-2">{{ store.pickedComic.title }}</span>
          <span class="text-red">作者：{{ store.pickedComic.author }}</span>
          <span class="text-gray" v-html="`分类：${store.pickedComic.categories}`"></span>
          <n-button
            v-if="store.pickedComic.isDownloaded === true"
            class="mr-auto mt-auto"
            size="tiny"
            @click="showComicDownloadDirInFileManager">
            打开下载目录
          </n-button>
        </div>
      </div>
    </div>
    <n-dropdown
      placement="bottom-start"
      trigger="manual"
      :x="dropdownX"
      :y="dropdownY"
      :options="dropdownOptions"
      :show="showDropdown"
      :on-clickoutside="() => (showDropdown = false)"
      @select="onDropdownSelect" />
  </div>
</template>

<style scoped>
.selection-container {
  @apply select-none;
}

.selection-container .selected {
  @apply bg-[rgb(204,232,255)];
}

.selection-container .downloaded {
  @apply bg-[rgba(24,160,88,0.16)];
}

.selection-container .downloading {
  @apply bg-[rgba(114,46,209,0.16)];
}

:deep(.n-checkbox__label) {
  @apply overflow-hidden whitespace-nowrap text-ellipsis;
}

:global(.selection-area) {
  @apply bg-[rgba(46,115,252,0.5)];
}
</style>
