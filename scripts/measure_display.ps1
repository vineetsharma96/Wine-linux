param(
    [Parameter(Mandatory)][string]$Adb,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$EnvironmentId,
    [string]$Distribution = "ubuntu",
    [ValidateRange(30, 18000)][int]$Frames = 360,
    [switch]$External,
    [switch]$InputProbe
)

$ErrorActionPreference = "Stop"
$package = if ($External) { "com.termux.x11" } else { "com.androidpc.environment.debug" }
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
function Device([string]$command) {
    $result = & $Adb shell $command
    if ($LASTEXITCODE -ne 0) { throw "ADB command failed ($LASTEXITCODE): $command" }
    return $result
}
function Capture([string]$name, [string]$command) {
    Device $command | Set-Content -Encoding utf8 (Join-Path $OutputDirectory $name)
}

Capture "layers.txt" "dumpsys SurfaceFlinger --list"
if (!(Select-String -SimpleMatch "SurfaceView[$package/" -Path (Join-Path $OutputDirectory "layers.txt"))) {
    throw "Open the $package display viewport before measuring"
}
Capture "processes-before.txt" "ps -A -o PID,PPID,PCPU,RSS,NAME"
Capture "device.txt" "getprop ro.product.model; getprop ro.build.version.release; uname -r; wm size; wm density; dumpsys display | grep renderFrameRate"
Device "dumpsys SurfaceFlinger --timestats -clear -enable" | Out-Null
$samples = 13
$cpu = Start-Job -ArgumentList $Adb,$samples -ScriptBlock {
    param($adbPath, $samples)
    $commands = '(i=0; while [ "$i" -lt {0} ]; do if [ -r /sys/class/kgsl/kgsl-3d0/gpubusy ]; then printf "gpu %s\n" "$(cat /sys/class/kgsl/kgsl-3d0/gpubusy)"; fi; sleep 1; i=$((i+1)); done) & gpu=$!; top -b -d 1 -n {0} -m 30; wait "$gpu"' -f $samples
    & $adbPath shell $commands
}
try {
    if ($External) {
        Device "rm -f /sdcard/Download/androidpc-display-done; echo $Frames > /sdcard/Download/androidpc-display-go" | Out-Null
        $deadline = (Get-Date).AddSeconds([math]::Ceiling($Frames / 30) + 25)
        while ((Device "if test -f /sdcard/Download/androidpc-display-done; then echo done; fi") -ne "done") {
            if ((Get-Date) -gt $deadline) { throw "Termux did not finish the armed workload" }
            Start-Sleep -Milliseconds 500
        }
    } else {
        if ($EnvironmentId -notmatch '^[A-Za-z0-9-]+$' -or $Distribution -notin @("ubuntu", "debian")) {
            throw "Specify the installed guest ID and distribution"
        }
        $apk = (Device "pm path $package" | Select-Object -First 1).Substring(8)
        $libs = $apk.Substring(0, $apk.LastIndexOf('/')) + "/lib/arm64"
        $base = "/data/user/0/$package/files"
        $root = "$base/environments/$Distribution/$EnvironmentId/rootfs"
        $guest = "export PROOT_LOADER={0}/libproot_loader.so PROOT_TMP_DIR={1}/dtmp LD_LIBRARY_PATH={0}:{1}/artifacts/lib DISPLAY=:1 DISPLAY_BENCH_FRAMES={3} DISPLAY_BENCH_INPUT={4}; exec {0}/libproot.so --kill-on-exit --link2symlink -0 -r {2} -b /proc -b /dev -b {1}/dtmp:/tmp -w /root /usr/bin/python3 -" -f $libs,$base,$root,$Frames,[int]$InputProbe.IsPresent
        $workload = Get-Content -Raw (Join-Path $PSScriptRoot "display_workload.py")
        $workload | & $Adb shell -T "run-as $package sh -c '$guest'" |
            Tee-Object (Join-Path $OutputDirectory "workload.txt")
        if ($LASTEXITCODE -ne 0) { throw "X11 workload failed" }
    }
    Capture "timestats.txt" "dumpsys SurfaceFlinger --timestats -dump"
    Capture "processes-after.txt" "ps -A -o PID,PPID,PCPU,RSS,NAME"
    Capture "memory.txt" "dumpsys meminfo $package"
    $stats = Get-Content -Raw (Join-Path $OutputDirectory "timestats.txt")
    $layers = $stats -split '(?m)(?=^layerName = )' | Where-Object { $_.Contains("SurfaceView[$package/") }
    foreach ($layer in $layers) {
        $layer -split '\r?\n' | Where-Object { $_ -match '^(layerName|averageFPS|totalFrames|droppedFrames) =' }
        foreach ($metric in @("present2present", "acquire2present")) {
            $histogram = [regex]::Match($layer, "(?m)^$metric histogram[^\n]*\n([^\r\n]+)").Groups[1].Value
            $buckets = [regex]::Matches($histogram, '(\d+)ms=(\d+)')
            $count = ($buckets | ForEach-Object { [int]$_.Groups[2].Value } | Measure-Object -Sum).Sum
            if (!$count) { continue }
            foreach ($percentile in @(50, 90, 99)) {
                $seen = 0
                foreach ($bucket in $buckets) {
                    $seen += [int]$bucket.Groups[2].Value
                    if ($seen -ge [math]::Ceiling($count * $percentile / 100)) {
                        Write-Output "$metric p$percentile = $($bucket.Groups[1].Value) ms"
                        break
                    }
                }
            }
        }
    }
} finally {
    Wait-Job -Job $cpu -Timeout 15 | Out-Null
    Receive-Job -Job $cpu | Set-Content -Encoding utf8 (Join-Path $OutputDirectory "cpu.txt")
    Remove-Job -Job $cpu -Force
    if ($External) {
        Device "rm -f /sdcard/Download/androidpc-display-go /sdcard/Download/androidpc-display-done" | Out-Null
    }
    Device "dumpsys SurfaceFlinger --timestats -disable" | Out-Null
}
