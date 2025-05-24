<script setup lang="ts">
import ComicCard from '../components/ComicCard.vue'
import { onMounted, ref, watch } from 'vue'
import { commands, DownloadAllFavoritesEvent, events, GetFavoriteSort } from '../bindings.ts'
import { useStore } from '../store.ts'
import { MessageReactive, SelectProps } from 'naive-ui'
import { useMessage } from 'naive-ui'
import DownloadAllFavoriteButton from '../components/DownloadAllFavoriteButton.vue'

type ProgressData = Extract<DownloadAllFavoritesEvent, { event: 'StartCreateDownloadTasks' }>['data'] & {
  progressMessage: MessageReactive
}

const message = useMessage()

const store = useStore()

const progresses = ref<Map<string, ProgressData>>(new Map())
let prepareMessage: MessageReactive | undefined

const sortOptions: SelectProps['options'] = [
  { label: '新到旧', value: 'TimeNewest' },
  { label: '旧到新', value: 'TimeOldest' },
]

const sortSelected = ref<GetFavoriteSort>('TimeNewest') // TODO: 添加一个选择器来控制这个值

async function getFavorite(sort: GetFavoriteSort, page: number) {
  console.log('getFavorite', sort, page)
  const result = await commands.getFavorite(sort, page)
  if (result.status === 'error') {
    console.error(result.error)
    prepareMessage?.destroy()
    progresses.value.forEach((progress) => {
      progress.progressMessage.destroy()
    })
    progresses.value.clear()
    return
  }
  store.getFavoriteResult = result.data
}

watch(
  () => store.currentTabName,
  async () => {
    if (store.getFavoriteResult !== undefined || store.currentTabName !== 'favorite') {
      return
    }
    await getFavorite(sortSelected.value, 1)
  },
  { immediate: true },
)

onMounted(async () => {
  await events.downloadAllFavoritesEvent.listen(({ payload }) => {
    if (payload.event === 'GettingFavorites') {
      prepareMessage = message.loading('正在获取收藏夹', { duration: 0 })
    } else if (payload.event === 'GettingComics' && prepareMessage !== undefined) {
      const { current, total } = payload.data
      prepareMessage.content = `正在获取收藏夹中的漫画(${current}/${total})`
    } else if (payload.event === 'EndGetComics' && prepareMessage !== undefined) {
      prepareMessage.type = 'success'
      prepareMessage.content = '成功获取收藏夹中所有的漫画'
      setTimeout(() => {
        prepareMessage?.destroy()
        prepareMessage = undefined
      }, 3000)
    } else if (payload.event === 'StartCreateDownloadTasks') {
      const { comicId, comicTitle, current, total } = payload.data
      progresses.value.set(comicId, {
        comicId,
        comicTitle,
        current,
        total,
        progressMessage: message.loading(
          () => {
            const progressData = progresses.value.get(comicId)
            if (progressData === undefined) return ''
            return `${progressData.comicTitle} 正在创建下载任务(${progressData.current}/${progressData.total})`
          },
          { duration: 0 },
        ),
      })
    } else if (payload.event === 'CreatingDownloadTask') {
      const { comicId, current } = payload.data
      const progressData = progresses.value.get(comicId)
      if (progressData) {
        progressData.current = current
      }
    } else if (payload.event === 'EndCreateDownloadTasks') {
      const { comicId } = payload.data
      const progressData = progresses.value.get(comicId)
      if (progressData) {
        progressData.progressMessage.type = 'success'
        progressData.progressMessage.content = `${progressData.comicTitle} 创建下载任务完成(${progressData.current}/${progressData.total})`
        setTimeout(() => {
          progressData.progressMessage.destroy()
          progresses.value.delete(comicId)
        }, 3000)
      }
    }
  })
})
</script>

<template>
  <div v-if="store.getFavoriteResult !== undefined" class="h-full flex flex-col gap-2">
    <div class="flex box-border px-2 pt-2">
      <n-input-group class="">
        <n-input-group-label size="small">排序方式</n-input-group-label>
        <n-select
          class="w-25"
          v-model:value="sortSelected"
          :options="sortOptions"
          :show-checkmark="false"
          size="small"
          @update-value="getFavorite($event, 1)" />
      </n-input-group>
      <download-all-favorite-button />
    </div>
    <div class="flex flex-col gap-row-2 overflow-auto box-border px-2">
      <comic-card
        v-for="{ id, title, author, categories, thumb, isDownloaded, comicDirName } in store.getFavoriteResult.docs"
        :key="id"
        :comic-id="id"
        :comic-title="title"
        :comic-author="author"
        :comic-categories="categories"
        :comic-downloaded="isDownloaded"
        :comic-dir-name="comicDirName"
        :thumb="thumb" />
    </div>
    <n-pagination
      class="box-border p-2 pt-0 mt-auto"
      :page-count="store.getFavoriteResult.pages"
      :page="store.getFavoriteResult.page"
      @update:page="getFavorite(sortSelected, $event)" />
  </div>
</template>
