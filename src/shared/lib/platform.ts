export const isMacPlatform = (): boolean => {
  if (typeof navigator === "undefined") return false;

  return (
    /Mac|iPhone|iPad|iPod/i.test(navigator.userAgent) ||
    /Mac/i.test(navigator.platform)
  );
};

export const isWindowsPlatform = (): boolean => {
  if (typeof navigator === "undefined") return false;

  return (
    /Windows|Win32|Win64/i.test(navigator.userAgent) ||
    /Win/i.test(navigator.platform)
  );
};
