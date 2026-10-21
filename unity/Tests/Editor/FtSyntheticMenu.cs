// Only copied to Assets/Editor in the isolated validation project.
using System.IO;
using UnityEditor;

public static class FtSyntheticMenu
{
    [MenuItem("Tools/VUA Synthetic/Face tracking")]
    [MenuItem("FT Synthetic/Apply")]
    private static void Patch()
    {
        File.AppendAllText("ft-menu-invoked.txt", "synthetic\n");
        if (File.Exists("ft-interactive-mode.txt")) EditorUtility.RequestScriptReload();
    }
    [MenuItem("Tools/VUA Synthetic/Face tracking", true)]
    private static bool Validate()
    {
        File.AppendAllText("ft-validator-count.txt", "v");
        return true;
    }
}
