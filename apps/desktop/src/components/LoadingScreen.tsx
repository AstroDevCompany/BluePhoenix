import logo from "../assets/brand/logo.png";

export function LoadingScreen() {
  return (
    <div className="boot-screen" role="status" aria-live="polite" aria-label="BluePhoenix is loading">
      <img src={logo} alt="BluePhoenix" />
    </div>
  );
}
