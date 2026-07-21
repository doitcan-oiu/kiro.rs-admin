import { useState } from 'react'
import {
  Activity, RefreshCw, UploadCloud, Settings, Key, Wand2, Eye, EyeOff, Copy,
  MoreHorizontal,
} from 'lucide-react'
import { useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { storage } from '@/lib/storage'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogDescription,
} from '@/components/ui/dialog'
import {
  DropdownMenu, DropdownMenuTrigger, DropdownMenuContent,
  DropdownMenuItem, DropdownMenuLabel,
} from '@/components/ui/dropdown-menu'
import {
  useLoadBalancingMode, useSetLoadBalancingMode,
} from '@/hooks/use-credentials'
import { useUpdateCheck } from '@/hooks/use-update-check'
import { updateAdminKey } from '@/api/credentials'
import { extractErrorMessage, generateApiKey } from '@/lib/utils'
import { ImageUpdateDialog } from '@/components/image-update-dialog'

/**
 * 顶栏右侧通用工具栏：负载均衡切换、刷新、在线更新、设置（Key 管理）。
 *
 * 与原 Dashboard 中的工具按钮等价，但全局 Tab 都可访问。刷新按钮会失效
 * 凭据/客户端 Key/统计三类查询，覆盖三个 Tab 的主要数据源。
 */
interface TopbarToolsProps {
  compact?: boolean
}

export function TopbarTools({ compact = false }: TopbarToolsProps) {
  const queryClient = useQueryClient()
  const { data: loadBalancingData, isLoading: isLoadingMode } = useLoadBalancingMode()
  const { mutate: setLoadBalancingMode, isPending: isSettingMode } = useSetLoadBalancingMode()
  const { data: updateCheck } = useUpdateCheck()

  const [imageUpdateOpen, setImageUpdateOpen] = useState(false)
  const [keyDialogOpen, setKeyDialogOpen] = useState(false)
  const [newKey, setNewKey] = useState('')
  const [showPlain, setShowPlain] = useState(false)
  const [updating, setUpdating] = useState(false)

  const handleRefresh = () => {
    queryClient.invalidateQueries({ queryKey: ['credentials'] })
    queryClient.invalidateQueries({ queryKey: ['client-keys'] })
    queryClient.invalidateQueries({ queryKey: ['stats'] })
    toast.success('已刷新')
  }

  const handleToggleLoadBalancing = () => {
    const cur = loadBalancingData?.mode || 'priority'
    const next = cur === 'priority' ? 'balanced' : 'priority'
    setLoadBalancingMode(next, {
      onSuccess: () => toast.success(`已切换到${next === 'priority' ? '优先级模式' : '均衡负载模式'}`),
      onError: (err) => toast.error(`切换失败: ${extractErrorMessage(err)}`),
    })
  }

  const openKeyDialog = () => {
    setNewKey('')
    setShowPlain(false)
    setKeyDialogOpen(true)
  }

  const handleUpdateKey = async (e: React.FormEvent) => {
    e.preventDefault()
    const key = newKey.trim()
    if (!key) {
      toast.error('新登录API密钥不能为空')
      return
    }
    setUpdating(true)
    try {
      await updateAdminKey({ newKey: key })
      storage.setApiKey(key)
      toast.success('登录API密钥已更新，已自动切换到新 Key')
      setKeyDialogOpen(false)
      setNewKey('')
    } catch (err) {
      toast.error(`更新失败: ${extractErrorMessage(err)}`)
    } finally {
      setUpdating(false)
    }
  }

  const controls = {
    handleRefresh,
    handleToggleLoadBalancing,
    isLoadingMode,
    isSettingMode,
    loadBalancingMode: loadBalancingData?.mode,
    openImageUpdate: () => setImageUpdateOpen(true),
    openKeyDialog,
    updateCheck,
  }

  return (
    <>
      {compact ? <CompactTools controls={controls} /> : <FullTools controls={controls} />}
      <ImageUpdateDialog open={imageUpdateOpen} onOpenChange={setImageUpdateOpen} />

      <Dialog
        open={keyDialogOpen}
        onOpenChange={(open) => { if (!updating) setKeyDialogOpen(open) }}
      >
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Key className="h-4 w-4" />
              修改登录API密钥
            </DialogTitle>
            <DialogDescription>
              用于登录此管理面板。修改后将自动更新本地存储的 Key，无需重新登录。
            </DialogDescription>
          </DialogHeader>
          <form onSubmit={handleUpdateKey} className="space-y-4 py-2">
            <div className="relative">
              <Input
                type={showPlain ? 'text' : 'password'}
                placeholder="输入或生成新的登录API密钥"
                value={newKey}
                onChange={(e) => setNewKey(e.target.value)}
                disabled={updating}
                autoFocus
                className="pr-20 font-mono text-[13px]"
              />
              <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-1.5">
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="pointer-events-auto h-7 w-7"
                  onClick={() => setShowPlain((v) => !v)}
                  disabled={updating}
                  title={showPlain ? '隐藏' : '显示'}
                >
                  {showPlain ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                </Button>
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  className="pointer-events-auto h-7 w-7"
                  onClick={async () => {
                    if (!newKey.trim()) {
                      toast.error('请先输入或生成 Key 再复制')
                      return
                    }
                    try {
                      await navigator.clipboard.writeText(newKey)
                      toast.success('已复制到剪贴板')
                    } catch {
                      toast.error('复制失败，请手动选择文本')
                    }
                  }}
                  disabled={updating}
                  title="复制"
                >
                  <Copy className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
            <div className="flex items-center justify-between gap-2">
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={() => {
                  const key = generateApiKey('sk-admin-')
                  setNewKey(key)
                  setShowPlain(true)
                }}
                disabled={updating}
              >
                <Wand2 className="h-3.5 w-3.5" />生成随机 Key
              </Button>
              <p className="text-[11px] text-muted-foreground">
                建议生成后立即复制保存，确认更新后即生效。
              </p>
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setKeyDialogOpen(false)} disabled={updating}>
                取消
              </Button>
              <Button type="submit" disabled={updating || !newKey.trim()}>
                {updating ? '更新中…' : '确认更新'}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  )
}

interface ToolControls {
  handleRefresh: () => void
  handleToggleLoadBalancing: () => void
  isLoadingMode: boolean
  isSettingMode: boolean
  loadBalancingMode?: 'priority' | 'balanced'
  openImageUpdate: () => void
  openKeyDialog: () => void
  updateCheck?: { hasUpdate: boolean; latestVersion: string; currentVersion: string }
}

function FullTools({ controls }: { controls: ToolControls }) {
  return (
    <>
      <LoadBalancingButton controls={controls} />
      <RefreshButton onRefresh={controls.handleRefresh} />
      <ImageUpdateButton controls={controls} />
      <KeySettingsMenu onOpenKeyDialog={controls.openKeyDialog} />
    </>
  )
}

function CompactTools({ controls }: { controls: ToolControls }) {
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="icon" title="更多操作">
          <MoreHorizontal className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-64">
        <DropdownMenuLabel>系统操作</DropdownMenuLabel>
        <DropdownMenuItem
          disabled={controls.isLoadingMode || controls.isSettingMode}
          onSelect={controls.handleToggleLoadBalancing}
        >
          <Activity />
          {controls.isLoadingMode
            ? '负载均衡加载中'
            : controls.loadBalancingMode === 'priority'
              ? '切换到均衡负载'
              : '切换到优先级'}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={controls.handleRefresh}>
          <RefreshCw />刷新数据
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={controls.openImageUpdate}>
          <UploadCloud />镜像在线更新
        </DropdownMenuItem>
        <DropdownMenuLabel>密钥管理</DropdownMenuLabel>
        <DropdownMenuItem onSelect={controls.openKeyDialog}>
          <Key />修改登录API密钥（管理面板登录）
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function LoadBalancingButton({ controls }: { controls: ToolControls }) {
  return (
    <Button
      variant="outline"
      size="sm"
      onClick={controls.handleToggleLoadBalancing}
      disabled={controls.isLoadingMode || controls.isSettingMode}
      title="切换负载均衡模式"
    >
      <Activity className="h-3.5 w-3.5" />
      <span className="hidden md:inline">
        {controls.isLoadingMode
          ? '加载中…'
          : controls.loadBalancingMode === 'priority'
            ? '优先级'
            : '均衡负载'}
      </span>
    </Button>
  )
}

function RefreshButton({ onRefresh }: { onRefresh: () => void }) {
  return (
    <Button variant="ghost" size="icon" onClick={onRefresh} title="刷新">
      <RefreshCw className="h-4 w-4" />
    </Button>
  )
}

function ImageUpdateButton({ controls }: { controls: ToolControls }) {
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={controls.openImageUpdate}
      title={imageUpdateTitle(controls.updateCheck)}
      className="relative"
    >
      <UploadCloud className="h-4 w-4" />
      {controls.updateCheck?.hasUpdate && <UpdateDot />}
    </Button>
  )
}

function KeySettingsMenu({ onOpenKeyDialog }: { onOpenKeyDialog: () => void }) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" title="设置">
          <Settings className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuLabel>密钥管理</DropdownMenuLabel>
        <DropdownMenuItem onSelect={onOpenKeyDialog}>
          <Key />修改登录API密钥（管理面板登录）
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

function imageUpdateTitle(updateCheck: ToolControls['updateCheck']) {
  if (!updateCheck?.hasUpdate) return '镜像在线更新'
  return `发现新版本 v${updateCheck.latestVersion}（当前 v${updateCheck.currentVersion}）`
}

function UpdateDot() {
  return (
    <span className="absolute right-1 top-1 inline-flex h-2 w-2 items-center justify-center">
      <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-400 opacity-75" />
      <span className="relative inline-flex h-2 w-2 rounded-full bg-red-500" />
    </span>
  )
}
