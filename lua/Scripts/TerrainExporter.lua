local lfs_loaded, lfs = pcall(require, "lfs")
local io = require("io")

if not lfs_loaded then
    lfs = {}
    function lfs.writedir()
        return "."
    end
end

package.path = package.path..";"..lfs.writedir().."/Scripts/?.lua"
local mp = require("msgpack")

function lshift(x, shift)
    local bit = require("luabit")
    return bit.blshift(x, shift)
end

if env == nil then
    env = {}
    function env.info(msg)
        print(msg)
    end
    function env.error(msg)
        print(msg)
    end
end

if land == nil then
    land = {}
    function land.getHeight(pos)
        return pos.x + pos.y
    end
end

if trigger == nil then
    trigger = { action = {} }
    function trigger.action.outText(text, displayTime, clearview)
        print(text)
    end
end

if timer == nil then
    timer = {}
    function timer.scheduleFunction(fn, arg, time)
        while fn(arg, time) ~= nil do end
    end
    function timer.getTime()
        return 0
    end
end

if Airbase == nil then
    Airbase = {}
    function Airbase.getByName(name)
        airbase = {}
        function airbase.getPoint()
            return { x = 0, y = 0, z = 0 }
        end
        return airbase
    end
end

local tile_sz = 16000 -- tile width and length in meters
local precision = 25 -- how many meters between terrain points
assert(tile_sz % precision == 0, "tile_sz must be a multiple of precision")

-- msgpack spec: https://github.com/msgpack/msgpack/blob/master/spec.md
local function fixmap(len)
    assert(len < 16)
    return string.char(0x80 + len)
end

local function array32(len)
    assert(len < 2^32)
    print(len)
    return "\xDD"..string.char(
        math.floor(len / lshift(1, 24)),
        math.floor((len / lshift(1, 16)) % 256),
        math.floor((len / lshift(1, 8)) % 256),
        math.floor(len % 256)
    )
end

local function export_tile(tile_x, tile_z, terrain)
    local filename = terrain.."_"..tile_sz.."_"..tile_x.."_"..tile_z..".pack"
    local filepath = lfs.writedir().."/tiles/"..filename
    local file = io.open(filepath, "r")
    if file ~= nil then
        env.info("Skipping "..filename..": file already exists")
        file:close()
        return nil
    end
    local rows = tile_sz / precision + 1
    local cols = rows
    local data = {}
    local zero = true
    local lowest = 999999
    for x = 0, rows - 1 do
        for z = 0, cols - 1 do
            local point = {
                x = tile_x * tile_sz + x * precision,
                y = tile_z * tile_sz + z * precision,
            }
            local height = land.getHeight(point)
            -- round the number to 1m precision to save space (with a small offset to not turn errors in the ocean into land)
            height = math.ceil(height - 0.05)
            data[x*cols + z + 1] = height
            if zero and (height < -0.01 or height > 0.01) then
                zero = false
            end
            if height < lowest then
                lowest = height
            end
        end
    end
    -- do not write data for zero-height tiles (we can assume those are water)
    if zero then
        env.info("Skipping tile ("..tile_x..", "..tile_z.."): all points are at sea level")
        offset = 0
        data = nil
    end
    -- normalize the data to the lowest point to save even more space
    if data ~= nil then
        for i, height in pairs(data) do
            data[i] = math.floor(height - lowest)
        end
    end
    local file, err = io.open(filepath, "wb")
    assert(file ~= nil, "Failed to open "..filename.." for writing: "..tostring(err))
    file:write(mp.pack({
        x = tile_x,
        z = tile_z,
        size = tile_sz,
        precision = precision,
        terrain = terrain,
        offset = lowest,
        data = data
    }))
    file:close()
end

local export_tiles = coroutine.create(function (terrain)
    local abort = false
    local start = trigger.misc.getZone("start")
    local end_ = trigger.misc.getZone("end")
    if start == nil then
        trigger.action.outText('Terrain export failed: must have a trigger called "start" at the bottom left corner of the map', 60, false)
        abort = true
    end
    if end_ == nil then
        trigger.action.outText('Terrain export failed: must have a trigger called "end" at the top right corner of the map', 60, false)
        abort = true
    end
    if abort then
        return false
    end
    local start = {
        x = math.floor(start.point.x / tile_sz),
        z = math.floor(start.point.z / tile_sz),
    }
    local end_ = {
        x = math.ceil(end_.point.x / tile_sz),
        z = math.ceil(end_.point.z / tile_sz),
    }
    assert(end_.x > start.x and end_.z > start.z, "end trigger must be northeast of start trigger")
    env.info("Start: "..(start.x)..", "..(start.z))
    env.info("End: "..(end_.x)..", "..(end_.z))
    local total = (end_.x - start.x + 1) * (end_.z - start.z + 1)
    local iters = 0
    for x = start.x, end_.x do
        for z = start.z, end_.z - 1 do
            trigger.action.outText("Tiles processed: "..iters.."/"..total.." ("..x..", "..z..")", 5, true)
            coroutine.yield()
            export_tile(x, z, terrain)
            iters = iters + 1
        end
    end
    trigger.action.outText("Terrain export finished", 60, true)
    return true
end)

-- local write_data = coroutine.create(function ()
--     local iters = 0
--     local size_x = math.abs(math.floor((max - start.x) / step))
--     local size_z = math.abs(math.floor((max - start.z) / step))
--     env.info("Terrain array size: "..size_x.."*"..size_z)
--     file:write(array32(size_x * size_z))
--     for x = max - step, start.x, -step do
--         print(x)
--         if iters % 10 == 0 then
--             trigger.action.outText("Terrain processed: "..x.."/"..(max - start.x), 5, true)
--             coroutine.yield()
--         end
--         for z = start.z, max - step, step do
--             print(x..", "..z)
--             file:write(mp.pack(land.getHeight({ x = x, y = z })))
--         end
--         iters = iters + 1
--     end
-- end)

-- file:write(fixmap(3)..(
--     mp.pack("start")..fixmap(2)..(
--         mp.pack("x")..mp.pack(start.x)..
--         mp.pack("y")..mp.pack(start.y)
--     )..
--     mp.pack("resolution")..mp.pack(step)..
--     mp.pack("data")
-- ))

-- local anapa = Airbase.getByName("Anapa-Vityazevo"):getPoint()
-- env.info("Anapa coords: "..anapa.x..", "..anapa.y..", "..anapa.z)

-- package.path  = package.path..";"..lfs.currentdir().."/LuaSocket/?.lua"
-- package.cpath = package.cpath..";"..lfs.currentdir().."/LuaSocket/?.dll"
-- package.path  = package.path..";"..lfs.currentdir().."/Scripts/?.lua"
-- env.info("Anapa: "..require("json"):encode(Airbase.getByName("Anapa-Vityazevo"):getDesc()))

timer.scheduleFunction(function(params, time)
    local continue, status = coroutine.resume(export_tiles, params.terrain)
    if continue then
        if status ~= nil then
            -- Finished gracefully
            return nil
        end
        -- Still more work to do
        return time + 0.01
    else
        trigger.action.outText("Error: "..status, 60, true)
        return nil
    end
end, { terrain = "caucasus" }, timer.getTime() + 1)
