import logo from "../../src-tauri/icons/128x128@2x.png";

/** The app's own icon, as the tray and the installer show it. */
export function BrandMark({ className = "size-6" }: { className?: string }) {
  return <img src={logo} alt="" aria-hidden draggable={false} className={`select-none ${className}`} />;
}
