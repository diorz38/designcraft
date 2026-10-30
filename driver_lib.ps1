param([int]$Port = 7991)
$ErrorActionPreference = 'Stop'
function Send([object]$obj) {
  $line = $obj | ConvertTo-Json -Compress -Depth 10
  $client = New-Object System.Net.Sockets.TcpClient('127.0.0.1', $Port)
  $s = $client.GetStream()
  $w = New-Object System.IO.StreamWriter($s); $w.NewLine = "`n"; $w.WriteLine($line); $w.Flush()
  $r = New-Object System.IO.StreamReader($s)
  $resp = $r.ReadLine()
  $client.Close()
  return $resp
}
$id = 100
function Req([string]$method, [object]$params) {
  $script:id++
  $resp = Send @{ id = $script:id; method = $method; params = $params }
  Write-Output "[$method] $resp"
  return $resp
}
