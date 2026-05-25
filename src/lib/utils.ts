// src/lib/utils.ts
import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"
import { convertFileSrc } from '@tauri-apps/api/core'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function getImageUrl(absolutePath: string): string {
  return convertFileSrc(absolutePath)
}

export function getThumbUrl(absolutePath: string): string {
  return convertFileSrc(absolutePath)
}
