<script setup lang="ts">
import { computed, ref } from 'vue'
import { ComicInSearchRespData, commands, Pagination, Sort } from '../bindings.ts'
import { useNotification } from 'naive-ui'
import ComicCard from '../components/ComicCard.vue'
import { ComicInfo } from '../types.ts'
import { SearchOutlined, ArrowRightOutlined } from '@vicons/antd'
import FloatLabelInput from '../components/FloatLabelInput.vue'

const notification = useNotification()

const sortOptions = [
  { label: '新到旧', value: 'TimeNewest' },
  { label: '旧到新', value: 'TimeOldest' },
  { label: '最多爱心', value: 'LikeMost' },
  { label: '最多指名', value: 'ViewMost' },
]

defineProps<{
  searchById: (comicId: string) => void
}>()

const searchInput = ref<string>('')
const searching = ref<boolean>(false)
const comicIdInput = ref<string>('')
const sortSelected = ref<Sort>('TimeNewest')
const comicInSearchPagination = ref<Pagination<ComicInSearchRespData>>()

const comicInfoPagination = computed<Pagination<ComicInfo> | undefined>(() => {
  const pagination = comicInSearchPagination.value
  if (pagination === undefined) {
    return undefined
  }
  return {
    ...pagination,
    docs: pagination.docs.map(({ _id, title, author, categories, thumb }) => ({
      _id,
      title,
      author,
      categories,
      thumb,
    })),
  }
})

async function searchByKeyword(keyword: string, sort: Sort, page: number, categories: string[]) {
  searching.value = true
  const result = await commands.searchComic(keyword, sort, page, categories)
  if (result.status === 'error') {
    searching.value = false
    notification.error({ title: '搜索失败', description: result.error })
    return
  }
  searching.value = false
  comicInSearchPagination.value = result.data
}
</script>

<template>
  <div class="h-full flex flex-col gap-2">
    <n-input-group class="box-border px-2 pt-2">
      <FloatLabelInput
        label="关键词"
        size="small"
        v-model:value="searchInput"
        clearable
        @keydown.enter="searchByKeyword(searchInput.trim(), sortSelected, 1, [])" />
      <n-select
        class="w-40"
        v-model:value="sortSelected"
        :options="sortOptions"
        :show-checkmark="false"
        size="small"
        @update-value="searchByKeyword(searchInput.trim(), $event, 1, [])" />
      <n-button
        :loading="searching"
        type="primary"
        size="small"
        class="w-15%"
        @click="searchByKeyword(searchInput.trim(), sortSelected, 1, [])">
        <template #icon>
          <n-icon size="22">
            <SearchOutlined />
          </n-icon>
        </template>
      </n-button>
    </n-input-group>

    <n-input-group class="box-border px-2">
      <FloatLabelInput
        label="漫画ID"
        size="small"
        v-model:value="comicIdInput"
        clearable
        @keydown.enter="searchById(comicIdInput.trim())" />
      <n-button type="primary" size="small" class="w-15%" @click="searchById(comicIdInput.trim())">
        <template #icon>
          <n-icon size="20">
            <ArrowRightOutlined />
          </n-icon>
        </template>
      </n-button>
    </n-input-group>

    <div v-if="comicInfoPagination !== undefined" class="flex flex-col gap-row-2 overflow-auto box-border px-2">
      <comic-card
        v-for="comicInfo in comicInfoPagination.docs"
        :key="comicInfo._id"
        :comic-info="comicInfo"
        :onClickItem="searchById" />
    </div>

    <n-pagination
      v-if="comicInfoPagination !== undefined"
      class="box-border p-2 pt-0 mt-auto"
      :page-count="comicInfoPagination.pages"
      :page="comicInfoPagination.page"
      @update:page="searchByKeyword(searchInput.trim(), sortSelected, $event, [])" />
  </div>
</template>
