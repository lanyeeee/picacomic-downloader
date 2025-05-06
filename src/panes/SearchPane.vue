<script setup lang="ts">
import { ref } from 'vue'
import { ComicInSearchRespData, commands, Pagination, Sort } from '../bindings.ts'
import ComicCard from '../components/ComicCard.vue'
import { SearchOutlined, ArrowRightOutlined } from '@vicons/antd'
import FloatLabelInput from '../components/FloatLabelInput.vue'
import { useStore } from '../store.ts'

const store = useStore()

const sortOptions = [
  { label: '新到旧', value: 'TimeNewest' },
  { label: '旧到新', value: 'TimeOldest' },
  { label: '最多爱心', value: 'LikeMost' },
  { label: '最多指名', value: 'ViewMost' },
]

const searchInput = ref<string>('')
const searching = ref<boolean>(false)
const comicIdInput = ref<string>('')
const sortSelected = ref<Sort>('TimeNewest')
const comicInSearchPagination = ref<Pagination<ComicInSearchRespData>>()

async function searchByKeyword(keyword: string, sort: Sort, page: number, categories: string[]) {
  searching.value = true
  const result = await commands.searchComic(keyword, sort, page, categories)
  if (result.status === 'error') {
    searching.value = false
    console.error(result.error)
    return
  }
  searching.value = false
  comicInSearchPagination.value = result.data
}

async function pickComic() {
  const result = await commands.getComic(comicIdInput.value.trim())
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  store.pickedComic = result.data
  store.currentTabName = 'chapter'
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
      <FloatLabelInput label="漫画ID" size="small" v-model:value="comicIdInput" clearable @keydown.enter="pickComic" />
      <n-button type="primary" size="small" class="w-15%" @click="pickComic">
        <template #icon>
          <n-icon size="20">
            <ArrowRightOutlined />
          </n-icon>
        </template>
      </n-button>
    </n-input-group>

    <div v-if="comicInSearchPagination !== undefined" class="flex flex-col gap-row-2 overflow-auto box-border px-2">
      <comic-card
        v-for="{ _id, title, author, categories, thumb } in comicInSearchPagination.docs"
        :key="_id"
        :comic-id="_id"
        :comic-title="title"
        :comic-author="author"
        :comic-categories="categories"
        :thumb="thumb" />
    </div>

    <n-pagination
      v-if="comicInSearchPagination !== undefined"
      class="box-border p-2 pt-0 mt-auto"
      :page-count="comicInSearchPagination.pages"
      :page="comicInSearchPagination.page"
      @update:page="searchByKeyword(searchInput.trim(), sortSelected, $event, [])" />
  </div>
</template>
