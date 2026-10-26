# Build assets/templates/covers/100_ai_base.designcraft from 012_quarter_stack.designcraft:
# keep the house slot frames + typography, replace the art with a full-bleed AI background
# image slot and an accent band, recolor over-art text to near-white.
$ErrorActionPreference = 'Stop'
$src = 'F:\GitHub\designcraft\assets\templates\covers\012_quarter_stack.designcraft'
$out = 'F:\GitHub\designcraft\assets\templates\covers\100_ai_base.designcraft'

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($src)
$entry = $zip.GetEntry('document.json')
$reader = New-Object System.IO.StreamReader($entry.Open())
$json = $reader.ReadToEnd()
$reader.Close()
$zip.Dispose()

$j = $json | ConvertFrom-Json
$sp = $j.spreads[0]
$PW = 595.2755905511812
$PH = 841.8897637795277

function Clone-Item($it) { $it | ConvertTo-Json -Depth 64 -Compress | ConvertFrom-Json }
function Set-Rect($item, [double]$x0, [double]$y0, [double]$x1, [double]$y1) {
    $a = $item.path.subpaths[0].anchors
    $a[0].p = [double[]]@($x0, $y0)
    $a[1].p = [double[]]@($x1, $y0)
    $a[2].p = [double[]]@($x1, $y1)
    $a[3].p = [double[]]@($x0, $y1)
}
function Set-StoryFill($item, $fill) {
    $st = $j.stories.PSObject.Properties | Where-Object Name -eq ([string]$item.content.story) | Select-Object -ExpandProperty Value
    foreach ($c in $st.chars) { $c.over.fill = $fill }
}

# 1. Swatches the palette binding expects (Primary/Accent/Dark, like the panel's palette call).
$j.swatches = @($j.swatches)
function Add-Swatch($name, [double]$r, [double]$g, [double]$b) {
    $j.swatches += [pscustomobject]@{
        name = $name
        value = [pscustomobject]@{ kind = 'color'; color = [pscustomobject]@{ model = 'rgb'; r = $r; g = $g; b = $b }; color_type = 'process' }
        locked = $false
        named = $true
    }
}
Add-Swatch 'Primary' 0.0 0.44705883 0.7372549      # #0072BC
Add-Swatch 'Accent' 0.9607843 0.67058825 0.09803922 # #F5AB19
Add-Swatch 'Dark' 0.07058824 0.14901961 0.2784314   # #122647

# 2. Full-bleed background image slot (reuses the hero's placed asset as the pre-apply
#    placeholder; template.page.apply runs file.place + object.fit over it).
$hero = $sp.items | Where-Object { $_.label -eq '#slot:image:hero' }
$bg = Clone-Item $hero
$bg.label = '#slot:image:background'
Set-Rect $bg 0.0 0.0 $PW $PH
$bgScaleX = [double]$PW / 1200.0
$bgScaleY = [double]$PH / 1200.0
$bg.content.xf = [double[]]@($bgScaleX, 0.0, 0.0, $bgScaleY, 0.0, 0.0)

# 3. Accent band behind the logo/agency strip (the template's one piece of furniture; the
#    palette binding recolors it).
$band0 = $sp.items | Where-Object { $_.id -eq 8 }
$band = Clone-Item $band0
$band.id = $j.nextId
$j.nextId = $j.nextId + 1
Set-Rect $band 0.0 772.0 $PW $PH
$band.fill.swatch = 'Accent'

# 4. Assemble: base paper rect, AI background, band, then the house text/logo slots.
$base = $sp.items | Where-Object { $_.id -eq 7 }
$keep = @($sp.items | Where-Object { $_.label -like '#slot*' -and $_.label -ne '#slot:image:hero' })
$sp.items = @($base) + @($bg) + @($band) + $keep

# 4b. The AI cover lifts the headline block higher (more art room below, and the title
#     frames must not share 012's exact shape — the distinct-designs test enforces it).
$titleFrame = $sp.items | Where-Object { $_.label -eq '#slot:text:title' }
Set-Rect $titleFrame 44.0 356.0 470.0 486.0
$title2Frame = $sp.items | Where-Object { $_.label -eq '#slot:text:title#2' }
Set-Rect $title2Frame 44.0 496.0 470.0 528.0
$yearFrame = $sp.items | Where-Object { $_.label -eq '#slot:text:year' }
Set-Rect $yearFrame 44.0 538.0 220.0 618.0
$year2Frame = $sp.items | Where-Object { $_.label -eq '#slot:text:year#2' }
Set-Rect $year2Frame 220.0 538.0 316.0 618.0

# 5. Over-art text reads near-white; the agency line sits on the accent band in dark ink.
foreach ($it in $sp.items) {
    if ($it.content.type -ne 'text') { continue }
    switch -Regex ($it.label) {
        '#slot:text:agency' { Set-StoryFill $it 'Stack Ink' }
        '#slot:text:' { Set-StoryFill $it 'Stack Paper' }
    }
}

$j.title = 'AI Base'

# 6. Write document.json back into a copy of the package (keeps preview + assets).
$jsonOut = $j | ConvertTo-Json -Depth 64
$tmpDoc = "$env:TEMP\ai_base_document.json"
[System.IO.File]::WriteAllText($tmpDoc, $jsonOut, [System.Text.UTF8Encoding]::new($false))

if (Test-Path $out) { Remove-Item $out }
Copy-Item $src $out
$zipW = [System.IO.Compression.ZipFile]::Open($out, 'Update')
$old = $zipW.GetEntry('document.json')
$old.Delete()
$new = $zipW.CreateEntry('document.json')
$stream = $new.Open()
$bytes = [System.IO.File]::ReadAllBytes($tmpDoc)
$stream.Write($bytes, 0, $bytes.Length)
$stream.Close()
$zipW.Dispose()

"DONE: $out ($((Get-Item $out).Length) bytes)"
