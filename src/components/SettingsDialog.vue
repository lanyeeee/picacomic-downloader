<script setup lang="ts">
import { commands } from '../bindings.ts'
import { path } from '@tauri-apps/api'
import { appDataDir } from '@tauri-apps/api/path'
import { useStore } from '../store.ts'

const store = useStore()

const showing = defineModel<boolean>('showing', { required: true })

async function showConfigInFileManager() {
  const configName = 'config.json'
  const configPath = await path.join(await appDataDir(), configName)
  const result = await commands.showPathInFileManager(configPath)
  if (result.status === 'error') {
    console.error(result.error)
  }
}
</script>

<template>
  <n-modal v-model:show="showing" v-if="store.config !== undefined">
    <n-dialog :showIcon="false" title="配置" @close="showing = false">
      <div class="flex flex-col gap-row-2">
        <n-input-group class="box-border">
          <n-input-group-label size="small">每个章节下载完成后休息</n-input-group-label>
          <n-input-number
            v-model:value="store.config.chapterDownloadInterval"
            :update-value-on-input="false"
            :min="0"
            size="small" />
          <n-input-group-label size="small">秒</n-input-group-label>
        </n-input-group>

        <n-tooltip placement="top" trigger="hover">
          <template #trigger>
            <n-checkbox v-model:checked="store.config.downloadWithAuthor" class="mr-auto">
              在漫画名前面附加作者名
            </n-checkbox>
          </template>
          [作者名] 漫画名
        </n-tooltip>

        <n-radio-group v-model:value="store.config.downloadFormat">
          <span class="mr-4">下载格式</span>
          <n-tooltip placement="top" trigger="hover">
            <div>原图不为jpg时，会自动转换为jpg</div>
            <template #trigger>
              <n-radio value="Jpeg">jpg</n-radio>
            </template>
          </n-tooltip>
          <n-tooltip placement="top" trigger="hover">
            <div>原图不为png时，会自动转换为png</div>
            <template #trigger>
              <n-radio value="Png">png</n-radio>
            </template>
          </n-tooltip>
          <n-tooltip placement="top" trigger="hover">
            <div>保持原图格式，不做任何转换</div>
            <template #trigger>
              <n-radio value="Original">原始格式</n-radio>
            </template>
          </n-tooltip>
        </n-radio-group>

        <n-button class="ml-auto mt-2" size="small" @click="showConfigInFileManager">打开配置目录</n-button>
      </div>
    </n-dialog>
  </n-modal>
</template>
