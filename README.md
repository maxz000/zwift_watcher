# zwift_watcher

Capture and analyze data from running Zwift client app.

With primary goal to provide actual data such as power, heart rate, cadence simultaneously for selected group of riders. 
For example, if you need to watch how all of your teammates doing.

Secondary trying to recreate and interpolate riders position on the map for given moment of time. 
Which may provide more accurate data for distance or time difference for riders in private "meetups",
automatically divide riders by groups (breakaway, peleton, gruppetto) etc.

# REST API
## Get basic info
latest world time and list of player ids in watchlist

### Request
`GET /`

    curl -i localhost:3030/
### Response

    HTTP/1.1 200 OK
    content-type: application/json
    content-length: 76
    date: Tue, 23 Mar 2021 07:28:30 GMT
    
    {"data":{"group_to_watch":[108934],"world_time":199877431690},"result":"ok"}

## Get watch group data
by default returns data at synchronized time for all players in group,

add `?latest=true` GET param if you want get latest available data

### Request
`GET /watch`

    curl -i localhost:3030/watch

or
`GET /watch?latest=true`

    curl -i "localhost:3030/watch?latest=true"

### Response
    HTTP/1.1 200 OK
    content-type: application/json
    content-length: 300
    date: Tue, 23 Mar 2021 07:29:50 GMT
    
    {"data":[{"cadence":56,"climbing":0,"distance":563,"group_id":0,"heading":1247938,"heartrate":125,"id":108934,"laps":0,"lean":992520,"power":115,"power_up":15,"road_position":10244300,"speed":8.905303888888888,"time":74,"world_time":199877475562,"x":1034.3646875,"y":-63.316513671875}],"result":"ok"}

## Add player to watch group
### Request
`POST /watch/add `

    curl -i -H 'Content-Type: application/json' -d '{"id": 108934}' localhost:3030/watch/add
### Response
    HTTP/1.1 200 OK
    content-type: application/json
    content-length: 36
    date: Tue, 23 Mar 2021 07:22:02 GMT
    
    {"data":{"id":108934},"result":"ok"}