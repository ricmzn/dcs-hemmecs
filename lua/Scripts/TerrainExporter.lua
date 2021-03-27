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
local start = { x = -8, z = 12 } -- in tiles, starting from map origin (ie. center of crimea in caucasus)
local tiles = { x = 10, z = 10 } -- how many tiles to create in each direction (note: +x = north, +y = east)
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
    local rows = tile_sz / precision + 1
    local cols = rows
    local data = {}
    local zero = true
    for x = 0, rows - 1 do
        for z = 0, cols - 1 do
            local point = {
                x = tile_x * tile_sz + x * precision,
                y = tile_z * tile_sz + z * precision,
            }
            local height = land.getHeight(point)
            -- round the number to save space in the tile files by allowing msgpack to use small ints
            -- offset by a small value to avoid rounding off-by-epsilon precision errors
            height = math.ceil(height - 0.05)
            data[x*cols + z + 1] = height
            if zero and (height < -0.01 or height > 0.01) then
                zero = false
            end
        end
    end
    -- if the file is empty, skip writing the data (but keep the metadata)
    if zero then
        env.info("Skipping tile ("..tile_x..", "..tile_z.."): all points are at sea level")
        data = nil
    end
    local filename = terrain.."_"..tile_sz.."_"..tile_x.."_"..tile_z..".pack"
    local file, err = io.open(lfs.writedir().."/tiles/"..filename, "wb")
    assert(file ~= nil, "Failed to open "..filename.." for writing: "..tostring(err))
    file:write(mp.pack({
        x = tile_x,
        z = tile_z,
        size = tile_sz,
        precision = precision,
        terrain = terrain,
        data = data
    }))
    file:close()
end

local export_tiles = coroutine.create(function (terrain)
    local iters = 0
    local total = tiles.x * tiles.z
    for x = start.x, start.x + tiles.x - 1 do
        for z = start.z, start.z + tiles.z - 1 do
            trigger.action.outText("Tiles processed: "..iters.."/"..total, 5, true)
            coroutine.yield()
            env.info("Processing tile ("..x..", "..z..")")
            export_tile(x, z, terrain)
            iters = iters + 1
        end
    end
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
        return time + 0.01
    else
        trigger.action.outText("Finished processing: "..status, 60, true)
        return nil
    end
end, { terrain = "caucasus" }, timer.getTime() + 1)
