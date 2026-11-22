/** A stopped/rebuilt preview can leave an open page referring to unavailable chunks. */
export function describeToolLoadError(error: unknown): { message: string; reload: boolean } {
  const failedDownload = error instanceof TypeError && /failed to fetch dynamically imported module|error loading dynamically imported module|importing a module script failed/i.test(error.message);
  if (failedDownload) {
    return {
      message: 'The tool interface could not be loaded. If using a browser preview, keep the preview server running, then reload the interface. Reloading clears unsaved values in open forms.',
      reload: true,
    };
  }
  return { message: `Could not load this tool: ${String(error)}`, reload: false };
}
