import type { Component } from "@builder.io/qwik";

/**
 * Checks if a component is present in the current component tree.
 * This function is analyzed at build time by qwik-analyzer.
 * 
 * @param component - The component reference to check for
 * @param injectedValue - Optional boolean value injected by qwik-analyzer at build time
 * @returns boolean indicating if the component is present
 */
export function isComponentPresent<T>(component: Component<T>, injectedValue?: boolean): boolean {
  // If we have an injected value from the analyzer, use it
  if (injectedValue !== undefined) {
    return injectedValue;
  }
  
  // This function is replaced at build time by the qwik-analyzer
  // In development, we return false as a fallback
  return false;
} 