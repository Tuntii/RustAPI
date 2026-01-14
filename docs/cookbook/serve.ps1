# Serve RustAPI Cookbook

if (Get-Command "mdbook" -ErrorAction SilentlyContinue) {
    Write-Host "Starting mdBook server..." -ForegroundColor Green
    mdbook serve --open
} else {
    Write-Host "X mdBook is not installed!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please install it by running:" -ForegroundColor Yellow
    Write-Host "  cargo install mdbook" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Once installed, run this script again." -ForegroundColor Gray
}
