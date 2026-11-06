local function nosolve(...)
    return ...
end

local value = nosolve(function(callback)
    while true do
        if not nosolve(function()
                return nosolve(function()
                    local v1_3 = nosolve(function()
                        return 5, 6
                    end)()
                    return v1_3 == nosolve(function()
                        return 5
                    end)()
                end)()
            end)() then break end
        if nosolve(function()
                return nosolve(function()
                    return callback()
                end)()
            end)() then
            return nosolve(function(arg1_2)
                while true do
                    if not nosolve(function()
                            return nosolve(function()
                                local v1_4 = nosolve(function()
                                    return 5, 6
                                end)()
                                return v1_4 == nosolve(function()
                                    return 5
                                end)()
                            end)()
                        end)() then break end
                    if nosolve(function()
                            return nosolve(function()
                                return arg1_2()
                            end)()
                        end)() then
                        return nosolve(function(arg1_3)
                            while true do
                                if not nosolve(function()
                                        return nosolve(function()
                                            local v1_5 = nosolve(function()
                                                return 5, 6
                                            end)()
                                            return v1_5 == nosolve(function()
                                                return 5
                                            end)()
                                        end)()
                                    end)() then break end
                                if nosolve(function()
                                        return nosolve(function()
                                            return arg1_3()
                                        end)()
                                    end)() then
                                    return nosolve(function(arg1_4)
                                        while true do
                                            if not nosolve(function()
                                                    return nosolve(function()
                                                        local v1_6 = nosolve(function()
                                                            return 5, 6
                                                        end)()
                                                        return v1_6 == nosolve(function()
                                                            return 5
                                                        end)()
                                                    end)()
                                                end)() then break end
                                            if nosolve(function()
                                                    return nosolve(function()
                                                        return arg1_4()
                                                    end)()
                                                end)() then
                                                return nosolve(function(arg1_5)
                                                    while true do
                                                        if not nosolve(function()
                                                                return nosolve(function()
                                                                    local v1_7 = nosolve(function()
                                                                        return 5, 6
                                                                    end)()
                                                                    return v1_7 == nosolve(function()
                                                                        return 5
                                                                    end)()
                                                                end)()
                                                            end)() then break end
                                                        if nosolve(function()
                                                                return nosolve(function()
                                                                    return arg1_5()
                                                                end)()
                                                            end)() then
                                                            return nosolve(function(arg1_6)
                                                                while true do
                                                                    if not nosolve(function()
                                                                            return nosolve(function()
                                                                                local v1_8 = nosolve(function()
                                                                                    return 5, 6
                                                                                end)()
                                                                                return v1_8 == nosolve(function()
                                                                                    return 5
                                                                                end)()
                                                                            end)()
                                                                        end)() then break end
                                                                    if nosolve(function()
                                                                            return nosolve(function()
                                                                                return arg1_6()
                                                                            end)()
                                                                        end)() then
                                                                        return nosolve(function()
                                                                            return arg1_6()
                                                                        end)()
                                                                    end
                                                                    return nosolve(function()
                                                                        return "Fail T2"
                                                                    end)()
                                                                end
                                                                return nosolve(function()
                                                                    return "Fail T1"
                                                                end)()
                                                            end)(arg1_5)
                                                        end
                                                        return nosolve(function()
                                                            return "Fail T2"
                                                        end)()
                                                    end
                                                    return nosolve(function()
                                                        return "Fail T1"
                                                    end)()
                                                end)(arg1_4)
                                            end
                                            return nosolve(function()
                                                return "Fail T2"
                                            end)()
                                        end
                                        return nosolve(function()
                                            return "Fail T1"
                                        end)()
                                    end)(arg1_3)
                                end
                                return nosolve(function()
                                    return "Fail T2"
                                end)()
                            end
                            return nosolve(function()
                                return "Fail T1"
                            end)()
                        end)(arg1_2)
                    end
                    return nosolve(function()
                        return "Fail T2"
                    end)()
                end
                return nosolve(function()
                    return "Fail T1"
                end)()
            end)(callback)
        end
        return nosolve(function()
            return "Fail T2"
        end)()
    end
    return nosolve(function()
        return "Fail T1"
    end)()
end)(function()
    return "Final" .. math.random(100000000, 999999999)
end)
print("out=", value)
