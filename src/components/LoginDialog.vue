<script setup lang="ts">
import { ref } from 'vue'
import { commands } from '../bindings.ts'
import { useMessage, useNotification } from 'naive-ui'
import FloatLabelInput from './FloatLabelInput.vue'

const message = useMessage()
const notification = useNotification()

const showing = defineModel<boolean>('showing', { required: true })
const token = defineModel<string>('token', { required: true })

const emailInput = ref<string>('')
const passwordInput = ref<string>('')

async function onLogin(email: string, password: string) {
  if (email === '') {
    message.error('请输入用户名')
    return
  }
  if (password === '') {
    message.error('请输入密码')
    return
  }
  const result = await commands.login(email, password)
  if (result.status === 'error') {
    notification.error({ title: '登录失败', description: result.error })
    return
  }
  message.success('登录成功')
  token.value = result.data
  showing.value = false
}
</script>

<template>
  <n-modal v-model:show="showing">
    <n-dialog
      :showIcon="false"
      title="账号登录"
      positive-text="登录"
      @positive-click="onLogin(emailInput, passwordInput)"
      @keydown.enter="onLogin(emailInput, passwordInput)"
      @close="showing = false">
      <div class="flex flex-col gap-2">
        <FloatLabelInput label="用户名" v-model:value="emailInput" />
        <FloatLabelInput label="密码" v-model:value="passwordInput" type="password" />
      </div>
    </n-dialog>
  </n-modal>
</template>
